use alloc::collections::VecDeque;
use alloc::boxed::Box;
use crate::sync::Spinlock;
use crate::sched::thread::{ThreadControlBlock, ThreadState};
use crate::sched;

struct EndpointState {
    senders: VecDeque<Box<ThreadControlBlock>>,
    receivers: VecDeque<Box<ThreadControlBlock>>,
}

pub struct Endpoint {
    state: Spinlock<EndpointState>,
}

impl Endpoint {
    pub fn new() -> Self {
        Self {
            state: Spinlock::new(EndpointState {
                senders: VecDeque::new(),
                receivers: VecDeque::new(),
            }),
        }
    }

    pub fn send(&self) {
        let mut state = self.state.lock();
        let mut current_lock = sched::CURRENT_THREAD.lock();
        let mut sender_thread = current_lock.take().expect("No current thread in send");

        if let Some(mut receiver) = state.receivers.pop_front() {
            // Fast path: Copy message to receiver
            receiver.ipc_buffer = sender_thread.ipc_buffer;
            receiver.state = ThreadState::Runnable;
            
            sched::RUN_QUEUE.lock().enqueue(receiver);
            
            // Sender continues
            sender_thread.state = ThreadState::Runnable;
            *current_lock = Some(sender_thread);
        } else {
            // Block sender
            sender_thread.state = ThreadState::Blocked;
            let sender_context_ptr = &mut sender_thread.context as *mut *mut crate::sched::thread::ThreadContext;
            state.senders.push_back(sender_thread);
            
            drop(current_lock);
            drop(state);
            
            sched::schedule_blocked(sender_context_ptr);
        }
    }

    pub fn recv(&self) {
        let mut state = self.state.lock();
        let mut current_lock = sched::CURRENT_THREAD.lock();
        let mut receiver_thread = current_lock.take().expect("No current thread in recv");

        if let Some(mut sender) = state.senders.pop_front() {
            // Fast path: copy from sender
            receiver_thread.ipc_buffer = sender.ipc_buffer;
            
            sender.state = ThreadState::Runnable;
            sched::RUN_QUEUE.lock().enqueue(sender);
            
            receiver_thread.state = ThreadState::Runnable;
            *current_lock = Some(receiver_thread);
        } else {
            // Block receiver
            receiver_thread.state = ThreadState::Blocked;
            let receiver_context_ptr = &mut receiver_thread.context as *mut *mut crate::sched::thread::ThreadContext;
            state.receivers.push_back(receiver_thread);
            
            drop(current_lock);
            drop(state);
            
            sched::schedule_blocked(receiver_context_ptr);
        }
    }
}
