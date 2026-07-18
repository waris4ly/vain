#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Invoke a syscall. Let's say SYS_LOG is 1.
    // Argument 1 in rdi, Arg 2 in rsi, Arg 3 in rdx.
    // Let's pass 42 in rdi to verify.
    unsafe {
        asm!(
            "syscall",
            in("rax") 1,    // syscall number
            in("rdi") 42,  // arg1
            options(nostack, preserves_flags)
        );
    }

    // Halt or infinite loop
    loop {
        unsafe { asm!("pause") }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
