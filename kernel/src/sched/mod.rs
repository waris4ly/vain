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

pub fn spawn_userspace_thread(
    priority: u8,
    user_entry: u64,
    user_stack: u64,
    cr3: u64,
    capabilities: alloc::vec::Vec<crate::cap::Capability>,
) {
    let kernel_stack = alloc::vec::Vec::<u8>::with_capacity(16384);
    let kernel_stack_top = kernel_stack.as_ptr() as u64 + 16384;
    core::mem::forget(kernel_stack);

    let mut tcb = Box::new(ThreadControlBlock::new_userspace(
        priority,
        user_entry,
        user_stack,
        kernel_stack_top,
        cr3,
    ));

    for cap in capabilities {
        tcb.cap_table.insert(cap);
    }

    RUN_QUEUE.lock().enqueue(tcb);
}

pub fn schedule() {
    let mut rq = RUN_QUEUE.lock();
    if let Some(next_thread) = rq.pick_next() {
        let mut current_lock = CURRENT_THREAD.lock();

        let mut prev_context_ptr_val: *mut ThreadContext = ptr::null_mut();
        let prev_context_ptr: *mut *mut ThreadContext;

        if let Some(mut current_thread) = current_lock.take() {
            prev_context_ptr = &mut current_thread.context as *mut *mut ThreadContext;

            if current_thread.state == thread::ThreadState::Runnable {
                rq.enqueue(current_thread);
            } else {
            }
        } else {
            prev_context_ptr = &mut prev_context_ptr_val as *mut *mut ThreadContext;
        }

        let next_context = next_thread.context;

        crate::arch::syscall::set_syscall_kernel_stack(next_thread.stack_top);
        crate::arch::gdt::set_kernel_stack(next_thread.stack_top);

        let next_cr3 = next_thread.cr3;
        *current_lock = Some(next_thread);

        drop(current_lock);
        drop(rq);

        unsafe {
            if next_cr3 != 0 {
                crate::mm::vmem::switch_page_table(next_cr3);
            }
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
        crate::arch::gdt::set_kernel_stack(next_thread.stack_top);

        let next_cr3 = next_thread.cr3;
        *current_lock = Some(next_thread);

        drop(current_lock);
        drop(rq);

        unsafe {
            if next_cr3 != 0 {
                crate::mm::vmem::switch_page_table(next_cr3);
            }
            context_switch::switch_context(prev_context_ptr, next_context);
        }
    } else {
        // Idle loop: enable interrupts and halt until a thread becomes runnable
        loop {
            unsafe {
                core::arch::asm!("sti; hlt", options(nomem, nostack));
            }
            // Check if any thread became runnable
            if RUN_QUEUE.lock().has_ready_threads() {
                // Return to schedule to pick it up properly without holding the lock
                break;
            }
        }
        // Retry schedule_blocked now that we have a runnable thread
        schedule_blocked(prev_context_ptr);
    }
}
