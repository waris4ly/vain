pub mod context_switch;
pub mod run_queue;
pub mod thread;

use crate::sync::Spinlock;
use alloc::boxed::Box;
use core::ptr;
use run_queue::RunQueue;
use thread::{ThreadContext, ThreadControlBlock};

pub static RUN_QUEUE: Spinlock<RunQueue> = Spinlock::new(RunQueue::new());
pub static CURRENT_THREAD: Spinlock<Option<Box<ThreadControlBlock>>> = Spinlock::new(None);

pub fn spawn_kernel_thread(priority: u8, entry: extern "C" fn() -> !, stack_top: u64) {
    let tcb = Box::new(ThreadControlBlock::new(priority, entry, stack_top));
    RUN_QUEUE.lock().enqueue(tcb);
}

pub fn spawn_userspace_thread(priority: u8, user_entry: u64, user_stack: u64) {
    // Allocate a kernel stack for this thread (16KB)
    let mut kernel_stack = alloc::vec::Vec::<u8>::with_capacity(16384);
    let kernel_stack_top = kernel_stack.as_ptr() as u64 + 16384;
    core::mem::forget(kernel_stack); // Leak the stack for now

    let tcb = Box::new(ThreadControlBlock::new_userspace(priority, user_entry, user_stack, kernel_stack_top));
    RUN_QUEUE.lock().enqueue(tcb);
}

pub fn schedule() {
    let mut rq = RUN_QUEUE.lock();
    if let Some(next_thread) = rq.pick_next() {
        let mut current_lock = CURRENT_THREAD.lock();

        let mut prev_context_ptr_val: *mut ThreadContext = ptr::null_mut();
        let prev_context_ptr: *mut *mut ThreadContext;

        if let Some(mut current_thread) = current_lock.take() {
            // Get a pointer to the context field inside the heap-allocated TCB
            prev_context_ptr = &mut current_thread.context as *mut *mut ThreadContext;

            if current_thread.state == thread::ThreadState::Runnable {
                rq.enqueue(current_thread);
            } else {
                // If blocked, we might store it somewhere else later.
                // For now, we just drop it (thread dies).
            }
        } else {
            // First ever schedule call: no previous thread to save context for.
            // We use a dummy local variable to receive the outgoing context and discard it.
            prev_context_ptr = &mut prev_context_ptr_val as *mut *mut ThreadContext;
        }

        let next_context = next_thread.context;

        // Put the incoming thread into the current slot
        crate::arch::syscall::set_syscall_kernel_stack(next_thread.stack_top);
        *current_lock = Some(next_thread);

        // Drop locks symmetrically before context switch
        drop(current_lock);
        drop(rq);

        unsafe {
            context_switch::switch_context(prev_context_ptr, next_context);
        }
    }
}

pub fn schedule_blocked(prev_context_ptr: *mut *mut ThreadContext) {
    let mut rq = RUN_QUEUE.lock();
    if let Some(next_thread) = rq.pick_next() {
        let mut current_lock = CURRENT_THREAD.lock();
        
        let next_context = next_thread.context;
        crate::arch::syscall::set_syscall_kernel_stack(next_thread.stack_top);
        
        *current_lock = Some(next_thread);
        
        drop(current_lock);
        drop(rq);
        
        unsafe {
            context_switch::switch_context(prev_context_ptr, next_context);
        }
    } else {
        crate::println!("[VAIN PANIC] No threads in RunQueue during schedule_blocked!");
        loop { unsafe { core::arch::asm!("hlt"); } }
    }
}
