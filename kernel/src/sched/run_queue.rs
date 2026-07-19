use alloc::boxed::Box;
use alloc::collections::VecDeque;

const NUM_PRIORITIES: usize = 32;

pub struct RunQueue {
    queues: [VecDeque<Box<crate::sched::thread::ThreadControlBlock>>; NUM_PRIORITIES],
}

impl RunQueue {
    pub const fn new() -> Self {
        const INIT: VecDeque<Box<crate::sched::thread::ThreadControlBlock>> = VecDeque::new();
        RunQueue {
            queues: [INIT; NUM_PRIORITIES],
        }
    }

    pub fn enqueue(&mut self, thread: Box<crate::sched::thread::ThreadControlBlock>) {
        let prio = thread.priority as usize;
        if prio < NUM_PRIORITIES {
            self.queues[prio].push_back(thread);
        }
    }

    pub fn pick_next(&mut self) -> Option<Box<crate::sched::thread::ThreadControlBlock>> {
        for i in (0..NUM_PRIORITIES).rev() {
            if let Some(thread) = self.queues[i].pop_front() {
                return Some(thread);
            }
        }
        None
    }
    pub fn has_ready_threads(&self) -> bool {
        for i in 0..NUM_PRIORITIES {
            if !self.queues[i].is_empty() {
                return true;
            }
        }
        false
    }
}
