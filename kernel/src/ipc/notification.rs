use crate::sched;
use crate::sched::thread::{ThreadControlBlock, ThreadState};
use crate::sync::Spinlock;
use alloc::boxed::Box;
use alloc::collections::VecDeque;

struct NotificationState {
    count: u64,
    waiters: VecDeque<Box<ThreadControlBlock>>,
}

pub struct Notification {
    state: Spinlock<NotificationState>,
}

impl Notification {
    pub fn new() -> Self {
        Self {
            state: Spinlock::new(NotificationState {
                count: 0,
                waiters: VecDeque::new(),
            }),
        }
    }

    pub fn signal(&self) {
        let mut state = self.state.lock();
        if let Some(mut waiter) = state.waiters.pop_front() {
            waiter.state = ThreadState::Runnable;
            sched::RUN_QUEUE.lock().enqueue(waiter);
        } else {
            state.count = state.count.saturating_add(1);
        }
    }

    pub fn wait(&self) {
        let mut state = self.state.lock();
        if state.count > 0 {
            state.count = state.count.saturating_sub(1);
        } else {
            let mut current_lock = sched::CURRENT_THREAD.lock();
            let mut waiter = current_lock.take().expect("No thread to wait");
            waiter.state = ThreadState::Blocked;
            let waiter_context_ptr =
                &mut waiter.context as *mut *mut crate::sched::thread::ThreadContext;
            state.waiters.push_back(waiter);

            drop(current_lock);
            drop(state);
            sched::schedule_blocked(waiter_context_ptr);
        }
    }
}
