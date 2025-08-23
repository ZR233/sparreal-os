use core::{
    alloc::Layout,
    fmt::Display,
    ops::Range,
    sync::atomic::{AtomicBool, Ordering},
};

use alloc::{alloc::alloc, collections::btree_map::BTreeMap};
use log::debug;

use crate::{
    irq,
    mem::{PhysAddr, mmu::RegionKind, region::boot_regions},
    platform::{CPUHardId, CPUId, cpu_hard_id, cpu_list, kstack_size},
    time::TimerData,
};

use super::once::OnceStatic;

static IS_INITED: AtomicBool = AtomicBool::new(false);
static HARD_TO_SOFT: OnceStatic<BTreeMap<CPUHardId, CPUId>> = OnceStatic::new(BTreeMap::new());
static SOFT_TO_HARD: OnceStatic<BTreeMap<CPUId, CPUHardId>> = OnceStatic::new(BTreeMap::new());
static PER_CPU: OnceStatic<BTreeMap<CPUHardId, PerCPU>> = OnceStatic::new(BTreeMap::new());

impl From<CPUHardId> for CPUId {
    fn from(value: CPUHardId) -> Self {
        unsafe { *(*HARD_TO_SOFT.get()).get(&value).unwrap() }
    }
}

impl From<CPUId> for CPUHardId {
    fn from(value: CPUId) -> Self {
        unsafe { *(*SOFT_TO_HARD.get()).get(&value).unwrap() }
    }
}

pub struct PerCPU {
    pub irq_chips: irq::CpuIrqChips,
    pub timer: TimerData,
    pub stack: Range<PhysAddr>,
}

/// 初始化PerCPU
///
/// # Safty
/// 只能在其他CPU启动前调用
pub unsafe fn setup_percpu() {
    let mut idx = 0;
    let cpu0 = cpu_hard_id();
    add_cpu(cpu0, idx);
    idx += 1;

    let cpus = cpu_list();
    for cpu in cpus {
        if cpu.cpu_id == cpu0 {
            continue;
        }
        add_cpu(cpu.cpu_id, idx);
        idx += 1;
    }
    IS_INITED.store(true, Ordering::SeqCst);
}

fn add_cpu(cpu: CPUHardId, idx: usize) {
    unsafe {
        let id = CPUId::from(idx);

        let stack_bottom = if idx == 0 {
            let region = boot_regions()
                .into_iter()
                .find(|o| matches!(o.kind, RegionKind::Stack))
                .expect("stack region not found!");

            region.range.start
        } else {
            let stack =
                alloc::alloc::alloc(Layout::from_size_align(kstack_size(), 0x1000).unwrap());
            PhysAddr::from(stack as usize - RegionKind::Other.va_offset())
        };

        (*PER_CPU.get()).insert(
            cpu,
            PerCPU {
                irq_chips: Default::default(),
                timer: Default::default(),
                stack: stack_bottom..stack_bottom + kstack_size(),
            },
        );
        (*HARD_TO_SOFT.get()).insert(cpu, id);
        (*SOFT_TO_HARD.get()).insert(id, cpu);
    }
}

pub fn cpu_global() -> &'static PerCPU {
    cpu_global_meybeuninit().expect("CPU global is not init!")
}

pub unsafe fn cpu_global_mut() -> &'static mut PerCPU {
    unsafe { cpu_global_mut_meybeunint().expect("CPU global is not init!") }
}

pub fn cpu_global_mut_meybeunint() -> Option<&'static mut PerCPU> {
    if !IS_INITED.load(Ordering::SeqCst) {
        return None;
    }
    let cpu = cpu_hard_id();
    unsafe { (*PER_CPU.get()).get_mut(&cpu) }
}

pub fn cpu_global_meybeuninit() -> Option<&'static PerCPU> {
    if !cpu_inited() {
        return None;
    }
    let cpu = cpu_hard_id();
    unsafe { (*PER_CPU.get()).get(&cpu) }
}

pub fn cpu_inited() -> bool {
    IS_INITED.load(Ordering::SeqCst)
}
