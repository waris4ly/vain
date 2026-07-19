#![no_std]
#![no_main]
#![forbid(unsafe_op_in_unsafe_fn)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]

extern crate alloc;

mod arch;
mod boot;
#[macro_use]
mod console;
pub mod mm;
pub mod process;
pub mod sched;
pub mod syscall;
pub mod sync;
mod ipc;
mod cap;

use core::fmt::Write;

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    kernel_main()
}

fn kernel_main() -> ! {
    boot::assert_base_revision_supported();

    arch::init();
    mm::init();

    let serial = console::serial::SerialPort::COM1;
    serial.initialize();

    println!("[VAIN] Memory primitives initialized");

    arch::apic::init();
    
    // Test Physical Allocator
    let frame1 = mm::frame_alloc::alloc_frame().expect("Failed to allocate frame1");
    let frame2 = mm::frame_alloc::alloc_frame().expect("Failed to allocate frame2");
    mm::frame_alloc::free_frame(frame2);
    
    // Initialize Syscalls
    arch::syscall::init();
    
    println!("[VAIN] Spawning init process...");
    process::load_init();
    
    println!("[VAIN] Starting scheduler...");
    sched::schedule();

    unreachable!("Scheduler should not return");
}

#[panic_handler]
fn on_panic(panic_info: &core::panic::PanicInfo) -> ! {
    let mut serial = console::serial::SerialPort::COM1;
    let _ = writeln!(serial, "[VAIN PANIC] {}", panic_info);
    arch::halt_loop()
}
