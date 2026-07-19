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
