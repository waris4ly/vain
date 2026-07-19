use vain_abi::syscall_numbers::*;
use crate::sched;

pub fn dispatch_syscall(sys_num: u64, arg1: u64, arg2: u64, _arg3: u64) -> u64 {
    match sys_num {
        SYS_EXIT => {
            crate::println!("[VAIN] Thread exiting with code {}", arg1);
            let mut current_lock = sched::CURRENT_THREAD.lock();
            if let Some(mut current) = current_lock.take() {
                current.state = sched::thread::ThreadState::Dead;
                let context_ptr = &mut current.context as *mut *mut sched::thread::ThreadContext;
                drop(current_lock);
                // Switch to next thread. Since state is Dead, it won't be enqueued.
                sched::schedule_blocked(context_ptr);
            } else {
                drop(current_lock);
            }
            unreachable!("Thread should have exited");
        }
        SYS_DEBUG_PRINT => {
            // Unsafe: we assume userspace passed a valid pointer and length.
            // In a real kernel, we must validate these against the process's page tables.
            unsafe {
                let bytes = core::slice::from_raw_parts(arg1 as *const u8, arg2 as usize);
                if let Ok(s) = core::str::from_utf8(bytes) {
                    crate::print!("{}", s);
                }
            }
            0
        }
        // SYS_SEND and SYS_RECV will be implemented here when needed
        _ => {
            crate::println!("[VAIN SYSCALL] Unknown syscall: {}", sys_num);
            !0
        }
    }
}
