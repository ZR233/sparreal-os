use core::{
    alloc::Layout,
    fmt::Debug,
    ops::{Deref, DerefMut},
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::{boxed::Box, string::String};
use log::trace;

use crate::{platform, task::schedule::*};

use super::{TaskConfig, TaskError};

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct TaskControlBlock(*mut u8);

unsafe impl Send for TaskControlBlock {}

impl From<*mut u8> for TaskControlBlock {
    fn from(value: *mut u8) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Pid(usize);

impl Pid {
    pub fn new() -> Self {
        static ITER: AtomicUsize = AtomicUsize::new(0);
        Self(ITER.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for Pid {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for Pid {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "{:?}", self.0)
    }
}

impl TaskControlBlock {
    pub(super) fn new<F>(entry: F, config: TaskConfig) -> Result<Self, TaskError>
    where
        F: FnOnce() + Send + 'static,
    {
        let entry_box = Box::new(entry);

        let buffer = NonNull::new(unsafe {
            alloc::alloc::alloc_zeroed(
                Layout::from_size_align(Self::tcb_size(config.stack_size), platform::page_size())
                    .unwrap(),
            )
        })
        .ok_or(TaskError::NoMemory)?;

        let pid = Pid::new();

        unsafe {
            let task_data = &mut *(buffer.as_ptr() as *mut TaskControlBlockData);
            task_data.pid = pid;
            task_data.stack_size = config.stack_size;
            task_data.priority = config.priority;
            task_data.name = config.name;
            task_data.state = TaskState::Idle;
            task_data.entry = Some(entry_box);
        }

        let mut task = Self(buffer.as_ptr());
        task.sp = task.stack_top() as usize;

        unsafe {
            task.sp -= platform::cpu_context_size();
            let ctx_ptr = task.sp as *mut u8;

            platform::cpu_context_set_sp(ctx_ptr, task.sp);
            platform::cpu_context_set_pc(ctx_ptr, task_entry as _);
        }
        Ok(task)
    }

    pub(super) fn new_main() -> Self {
        let entry_box = Box::new(|| {});

        let buffer = NonNull::new(unsafe {
            alloc::alloc::alloc_zeroed(
                Layout::from_size_align(Self::tcb_size(0), platform::page_size()).unwrap(),
            )
        })
        .ok_or(TaskError::NoMemory)
        .expect("main task no memory");

        let pid = Pid::new();

        unsafe {
            let task_data = &mut *(buffer.as_ptr() as *mut TaskControlBlockData);
            task_data.pid = pid;
            task_data.stack_size = 0;
            task_data.priority = 0;
            task_data.name = "Main".into();
            task_data.state = TaskState::Running;
            task_data.entry = Some(entry_box);
        }
        Self(buffer.as_ptr())
    }

    fn tcb_size(stack_size: usize) -> usize {
        size_of::<TaskControlBlockData>() + stack_size
    }

    fn stack_bottom(&self) -> *mut u8 {
        unsafe { self.0.add(size_of::<TaskControlBlockData>()) }
    }

    fn stack_top(&self) -> *mut u8 {
        unsafe { self.stack_bottom().add(self.stack_size) }
    }

    pub(super) unsafe fn drop(self) {
        let size = Self::tcb_size(self.stack_size);

        unsafe {
            alloc::alloc::dealloc(
                self.0,
                Layout::from_size_align_unchecked(size, platform::page_size()),
            )
        };
    }

    fn addr(&self) -> *mut u8 {
        self.0 as _
    }

    pub(super) fn switch_to(&self, next: &TaskControlBlock) {
        trace!("switch {} -> {}", self.name, next.name);
        set_current(next);
        match self.state {
            TaskState::Stopped => finished_push(*self),
            _ => idle_push(*self),
        }

        unsafe {
            platform::cpu_context_switch(self.addr(), next.addr());
        }
    }
}

impl Deref for TaskControlBlock {
    type Target = TaskControlBlockData;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.0 as *mut TaskControlBlockData) }
    }
}

impl DerefMut for TaskControlBlock {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.0 as *mut TaskControlBlockData) }
    }
}

pub struct TaskControlBlockData {
    pub pid: Pid,
    pub name: String,
    pub priority: usize,
    pub stack_size: usize,
    pub entry: Option<Box<dyn FnOnce()>>,
    pub state: TaskState,
    pub sp: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskState {
    Idle,
    Running,
    Suspend,
    Stopped,
}

extern "C" fn task_entry() -> ! {
    let mut task = current();

    if let Some(entry) = task.entry.take() {
        entry();
        task.state = TaskState::Stopped;
    }
    schedule();
    unreachable!("task exited!");
}

pub fn current() -> TaskControlBlock {
    let ptr = unsafe { platform::get_current_tcb_addr() };
    TaskControlBlock::from(ptr)
}

pub fn set_current(tcb: &TaskControlBlock) {
    unsafe { platform::set_current_tcb_addr(tcb.addr()) };
}
