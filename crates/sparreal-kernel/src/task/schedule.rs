use alloc::collections::vec_deque::VecDeque;
use log::debug;
use spin::Mutex;

use crate::platform;

use super::tcb::{TaskControlBlock, TaskState, current};

static IDLE: Mutex<VecDeque<TaskControlBlock>> = Mutex::new(VecDeque::new());
static FINISHED: Mutex<VecDeque<TaskControlBlock>> = Mutex::new(VecDeque::new());

pub fn schedule() {
    let idle = idle_pop();
    if let Some(mut idle) = idle {
        let mut cu = current();
        if matches!(cu.state, TaskState::Running) {
            cu.state = TaskState::Suspend;
        }
        idle.state = TaskState::Running;

        cu.switch_to(&idle);
    } else {
        debug!("No task idle");
        loop {
            platform::wait_for_interrupt();
        }
    }
}

pub fn idle_push(tcb: TaskControlBlock) {
    IDLE.lock().push_back(tcb);
}

pub fn idle_pop() -> Option<TaskControlBlock> {
    let mut g = IDLE.lock();
    while let Some(one) = g.pop_front() {
        if matches!(one.state, TaskState::Stopped) {
            unsafe { one.drop() };
            continue;
        }
        return Some(one);
    }
    None
}

pub fn finished_push(tcb: TaskControlBlock) {
    FINISHED.lock().push_back(tcb);
}

pub fn suspend() {
    let mut current = current();
    current.state = TaskState::Suspend;
    schedule();
}
