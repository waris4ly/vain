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
mod cap;
mod ipc;
pub mod mm;
pub mod process;
pub mod sched;
pub mod sync;
pub mod syscall;

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

    console::framebuffer::clear_screen();

    println!("[VAIN] Memory primitives initialized");

    arch::apic::init();

    // Test Physical Allocator
    let _frame1 = mm::frame_alloc::alloc_frame().expect("Failed to allocate frame1");
    let frame2 = mm::frame_alloc::alloc_frame().expect("Failed to allocate frame2");
    mm::frame_alloc::free_frame(frame2);

    // Initialize Syscalls
    arch::syscall::init();

    // Create IRQ1 Notification
    let irq1_notif = alloc::sync::Arc::new(ipc::notification::Notification::new());
    *IRQ1_NOTIFICATION.lock() = Some(irq1_notif.clone());

    println!("[VAIN] Spawning init process...");
    process::spawn_process("init", alloc::vec::Vec::new());

    println!("[VAIN] Spawning ps2-keyboard driver...");
    // Pass the notification capability to the driver
    process::spawn_process(
        "ps2-keyboard",
        alloc::vec![crate::cap::Capability::Notification(irq1_notif)],
    );

    println!("[VAIN] Starting scheduler...");
    sched::schedule();

    unreachable!("Scheduler should not return");
}

pub static IRQ1_NOTIFICATION: crate::sync::Spinlock<
    Option<alloc::sync::Arc<ipc::notification::Notification>>,
> = crate::sync::Spinlock::new(None);

#[panic_handler]
fn on_panic(panic_info: &core::panic::PanicInfo) -> ! {
    let mut serial = console::serial::SerialPort::COM1;
    let _ = writeln!(serial, "[VAIN PANIC] {}", panic_info);
    arch::halt_loop()
}
