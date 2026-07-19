use core::arch::asm;
use vain_abi::syscall_numbers::*;

#[inline]
pub fn sys_exit(code: u64) -> ! {
    unsafe {
        asm!(
            "syscall",
            in("rax") SYS_EXIT,
            in("rdi") code,
            options(noreturn)
        )
    }
}

#[inline]
pub fn sys_debug_print(s: &str) {
    unsafe {
        asm!(
            "syscall",
            in("rax") SYS_DEBUG_PRINT,
            in("rdi") s.as_ptr() as u64,
            in("rsi") s.len() as u64,
            out("rcx") _, out("r11") _, // syscall clobbers
            options(nostack, preserves_flags)
        )
    }
}

#[inline]
pub fn sys_send(endpoint_handle: u64, msg: &vain_abi::ipc_message::IpcMessage) {
    unsafe {
        asm!(
            "syscall",
            in("rax") SYS_SEND,
            in("rdi") endpoint_handle,
            in("rsi") msg as *const _ as u64,
            out("rcx") _, out("r11") _,
            options(nostack, preserves_flags)
        )
    }
}

#[inline]
pub fn sys_recv(endpoint_handle: u64, msg: &mut vain_abi::ipc_message::IpcMessage) {
    unsafe {
        asm!(
            "syscall",
            in("rax") SYS_RECV,
            in("rdi") endpoint_handle,
            in("rsi") msg as *mut _ as u64,
            out("rcx") _, out("r11") _,
            options(nostack, preserves_flags)
        )
    }
}

#[inline]
pub fn sys_port_in(port: u16) -> u8 {
    let mut value: u64;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") SYS_PORT_IN => value,
            in("rdi") port as u64,
            out("rcx") _, out("r11") _,
            options(nostack, preserves_flags)
        )
    }
    value as u8
}

#[inline]
pub fn sys_port_out(port: u16, value: u8) {
    unsafe {
        asm!(
            "syscall",
            in("rax") SYS_PORT_OUT,
            in("rdi") port as u64,
            in("rsi") value as u64,
            out("rcx") _, out("r11") _,
            options(nostack, preserves_flags)
        )
    }
}

#[inline]
pub fn sys_wait_irq(irq_handle: u64) {
    unsafe {
        asm!(
            "syscall",
            in("rax") SYS_WAIT_IRQ,
            in("rdi") irq_handle,
            out("rcx") _, out("r11") _,
            options(nostack, preserves_flags)
        )
    }
}
