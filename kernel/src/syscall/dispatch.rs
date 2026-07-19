use crate::sched;
use vain_abi::syscall_numbers::*;

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
            const MAX_PRINT_SIZE: usize = 4096;

            if arg2 > MAX_PRINT_SIZE as u64 {
                return !0;
            }

            let len = arg2 as usize;
            if len == 0 {
                return 0;
            }

            let ptr = arg1 as *const u8;
            if ptr.is_null() {
                return !0;
            }

            unsafe {
                let start_page = (arg1 & !0xFFF) as u64;
                let end_page = ((arg1 + arg2 + 0xFFF) & !0xFFF) as u64;

                for page in (start_page..end_page).step_by(4096) {
                    if !crate::mm::vmem::is_mapped(page) {
                        return !0;
                    }
                }

                let bytes = core::slice::from_raw_parts(ptr, len);
                if let Ok(s) = core::str::from_utf8(bytes) {
                    crate::print!("{}", s);
                } else {
                    return !0;
                }
            }
            0
        }
        SYS_SEND => {
            let handle = arg1;
            let msg_ptr = arg2 as *const vain_abi::ipc_message::IpcMessage;

            if msg_ptr.is_null() {
                return !0;
            }

            let msg_addr = msg_ptr as u64;
            let msg_size = core::mem::size_of::<vain_abi::ipc_message::IpcMessage>() as u64;

            unsafe {
                let start_page = (msg_addr & !0xFFF) as u64;
                let end_page = ((msg_addr + msg_size + 0xFFF) & !0xFFF) as u64;

                for page in (start_page..end_page).step_by(4096) {
                    if !crate::mm::vmem::is_mapped(page) {
                        return !0;
                    }
                }
            }

            let endpoint = {
                let lock = sched::CURRENT_THREAD.lock();
                if let Some(current) = lock.as_ref() {
                    match current.cap_table.get(handle) {
                        Some(crate::cap::Capability::Endpoint(ep)) => Some(ep),
                        _ => None,
                    }
                } else {
                    None
                }
            };

            if let Some(ep) = endpoint {
                unsafe {
                    let mut lock = sched::CURRENT_THREAD.lock();
                    let current = lock.as_mut().unwrap();
                    core::ptr::copy_nonoverlapping(msg_ptr, &mut current.ipc_buffer as *mut _, 1);
                }
                ep.send();
                0
            } else {
                !0
            }
        }
        SYS_RECV => {
            let handle = arg1;
            let msg_ptr = arg2 as *mut vain_abi::ipc_message::IpcMessage;

            if msg_ptr.is_null() {
                return !0;
            }

            let msg_addr = msg_ptr as u64;
            let msg_size = core::mem::size_of::<vain_abi::ipc_message::IpcMessage>() as u64;

            unsafe {
                let start_page = (msg_addr & !0xFFF) as u64;
                let end_page = ((msg_addr + msg_size + 0xFFF) & !0xFFF) as u64;

                for page in (start_page..end_page).step_by(4096) {
                    if !crate::mm::vmem::is_mapped(page) {
                        return !0;
                    }
                }
            }

            let endpoint = {
                let lock = sched::CURRENT_THREAD.lock();
                if let Some(current) = lock.as_ref() {
                    match current.cap_table.get(handle) {
                        Some(crate::cap::Capability::Endpoint(ep)) => Some(ep),
                        _ => None,
                    }
                } else {
                    None
                }
            };

            if let Some(ep) = endpoint {
                ep.recv();
                unsafe {
                    let mut lock = sched::CURRENT_THREAD.lock();
                    let current = lock.as_mut().unwrap();
                    core::ptr::copy_nonoverlapping(&current.ipc_buffer as *const _, msg_ptr, 1);
                }
                0
            } else {
                !0
            }
        }
        SYS_PORT_IN => {
            let port = arg1 as u16;
            let value: u8;
            unsafe {
                core::arch::asm!(
                    "in al, dx",
                    out("al") value,
                    in("dx") port,
                    options(nomem, nostack, preserves_flags)
                );
            }
            value as u64
        }
        SYS_PORT_OUT => {
            let port = arg1 as u16;
            let value = arg2 as u8;
            unsafe {
                core::arch::asm!(
                    "out dx, al",
                    in("dx") port,
                    in("al") value,
                    options(nomem, nostack, preserves_flags)
                );
            }
            0
        }
        SYS_WAIT_IRQ => {
            let handle = arg1;
            // Look up notification
            let notification = {
                let lock = sched::CURRENT_THREAD.lock();
                if let Some(current) = lock.as_ref() {
                    match current.cap_table.get(handle) {
                        Some(crate::cap::Capability::Notification(notif)) => Some(notif),
                        _ => None,
                    }
                } else {
                    None
                }
            };

            if let Some(notif) = notification {
                notif.wait();
                0
            } else {
                crate::println!("[VAIN] SYS_WAIT_IRQ: Invalid notification handle");
                !0
            }
        }
        _ => {
            crate::println!("[VAIN SYSCALL] Unknown syscall: {}", sys_num);
            !0
        }
    }
}
