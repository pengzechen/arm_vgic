
// use serde::{
//     de::{self, Visitor},
//     Deserialize, Deserializer, Serialize,
// };
use core::{fmt::Error, ops::Range};
extern crate alloc;
use alloc::sync::Arc;

#[derive(Copy, Clone, Debug)]
pub enum IrqState {
    IrqSInactive,
    IrqSPend,
    IrqSActive,
    IrqSPendActive,
}

impl IrqState {
    pub fn num_to_state(num: usize) -> IrqState {
        match num {
            0 => IrqState::IrqSInactive,
            1 => IrqState::IrqSPend,
            2 => IrqState::IrqSActive,
            3 => IrqState::IrqSPendActive,
            _ => panic!("num_to_state: illegal irq state"),
        }
    }

    pub fn to_num(self) -> usize {
        match self {
            IrqState::IrqSInactive => 0,
            IrqState::IrqSPend => 1,
            IrqState::IrqSActive => 2,
            IrqState::IrqSPendActive => 3,
        }
    }
}

pub fn gic_is_priv(int_id: usize) -> bool {
    int_id < 32
}

/* ========== VM =========== */

use alloc::vec::Vec;
pub struct Vm {
    pub id: usize,
    pub vcpu_list: Vec<Vcpu>
}

impl Vm {
    pub fn id(&self) -> usize {
        self.id
    }
    pub fn vcpu(&self, id :usize) -> Option<Vcpu> {
        self.vcpu_list.get(id).copied()
    }
    pub fn cpu_num(&self) -> usize {
        self.vcpu_list.len()
    }
    pub fn has_interrupt(&self, id: usize) -> bool {false}
    pub fn emu_has_interrupt(&self, id: usize) -> bool {false}
    /*
    pub fn emu_has_interrupt(&self, int_id: usize) -> bool {
        for emu_dev in self.config().emulated_device_list() {
            if int_id == emu_dev.irq_id {
                return true;
            }
        }
        false
    }
    */
    pub fn vgic(&self) -> Vgic { 
        Vgic::new(1, 1, 1)
    }

    #[inline]
    pub fn vcpu_list(&self) -> &[Vcpu] {
        &self.vcpu_list
    }

    pub fn vcpuid_to_pcpuid(&self, vcpuid: usize) -> Result<usize, ()> {
        self.vcpu_list().get(vcpuid).map(|vcpu| vcpu.phys_id()).ok_or(())
    }

    pub fn pcpuid_to_vcpuid(&self, pcpuid: usize) -> Result<usize, ()> {
        for vcpu in self.vcpu_list() {
            if vcpu.phys_id() == pcpuid {
                return Ok(vcpu.id());
            }
        }
        Err(())
    }

    pub fn vcpu_to_pcpu_mask(&self, mask: usize, len: usize) -> usize {
        let mut pmask = 0;
        for i in 0..len {
            let shift = self.vcpuid_to_pcpuid(i);
            if mask & (1 << i) != 0 && shift.is_ok() {
                pmask |= 1 << shift.unwrap();
            }
        }
        pmask
    }

    pub fn pcpu_to_vcpu_mask(&self, mask: usize, len: usize) -> usize {
        let mut pmask = 0;
        for i in 0..len {
            let shift = self.pcpuid_to_vcpuid(i);
            if mask & (1 << i) != 0 && shift.is_ok() {
                pmask |= 1 << shift.unwrap();
            }
        }
        pmask
    }
}


/* =========== VCPU ========== */

#[derive(Copy, Clone, Debug)] 
pub struct Vcpu {
    id     : usize,
    phys_id: usize,
    vm_id  : usize,
}

use crate::vgic::Vgic;
impl Vcpu {
    pub fn vm(&self) -> Option<Arc<Vm>> { Option::None }
    
    
    pub fn id(&self) -> usize { self.id }
    pub fn vm_id(&self) ->usize { self.phys_id }
    pub fn phys_id(&self) ->usize {0}
    
}


/* ========= Current cpu (pcpu) ============ */

pub struct VcpuArray {
    array: [Option<Vcpu>; 64],
    len: usize,
}

impl VcpuArray {
    pub const fn new() -> Self {
        Self {
            array: [const { None }; 64],
            len: 0,
        }
    }

    #[inline]
    pub fn pop_vcpu_through_vmid(&self, vm_id: usize) -> Option<&Vcpu> {
        match self.array.get(vm_id) {
            Some(vcpu) => vcpu.as_ref(),
            None => None,
        }
    }
}

pub struct Pcpu {
    pub active_vcpu  : Option<Vcpu>,
    pub vcpu_array   : VcpuArray
}

pub fn current_cpu() -> Pcpu {
    Pcpu {
        active_vcpu: None,
        vcpu_array: VcpuArray::new()
    }
}

impl Pcpu {
    pub fn id(&self) -> usize { 0 }
    pub fn get_gpr(&self, idx: usize) -> usize {0} 
    pub fn set_gpr(&self, idx: usize, val: usize) {}
    pub fn active_vcpu(&self) -> Vcpu {Vcpu { id: 0, vm_id: 0, phys_id: 0 }}
}


pub fn active_vm_id() -> usize {0}
pub fn active_vm() -> Option<Vm> { Option::None }
pub fn active_vcpu_id() -> usize {0}
pub fn active_vm_ncpu() -> usize {0}



#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EmuDeviceType {
    // Variants representing different emulator device types.
    EmuDeviceTConsole = 0,
    EmuDeviceTGicd = 1,
}

pub trait EmuDev {
    fn emu_type(&self) -> EmuDeviceType;

    fn address_range(&self) -> Range<usize>;

    fn handler(&self, emu_ctx: &EmuContext) -> bool;
}

/* ================ IPI relevant =============== */

#[derive(Copy, Clone, Debug)] pub enum InitcEvent {
    VgicdGichEn,
    VgicdSetEn,
    VgicdSetAct,
    VgicdSetPend,
    VgicdSetPrio,
    VgicdSetTrgt,
    VgicdSetCfg,
    VgicdRoute,
    Vgicdinject,
    None,
}

#[derive(Copy, Clone)] pub struct IpiInitcMessage {
    pub event: InitcEvent,
    pub vm_id: usize,
    pub int_id: u16,
    pub val: u8,
}

#[derive(Clone)] pub enum IpiInnerMsg {
    Initc(IpiInitcMessage),
    None,
}

pub enum IpiType {None}

pub struct IpiMessage {
    pub ipi_type: IpiType,
    pub ipi_message: IpiInnerMsg,
}

/* ================= ctx =============== */
#[derive(Debug, Clone, Copy)] pub struct EmuContext {
    pub address: usize,
    pub width: usize,
    pub write: bool,
    pub sign_ext: bool,
    pub reg: usize,
    pub reg_width: usize,
}


