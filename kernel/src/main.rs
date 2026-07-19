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
mod mm;
mod process;
mod sched;
mod sync;

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

    // Enable hardware interrupts
    arch::enable_interrupts();

    arch::syscall::init();

    // Allocate a kernel stack for Syscalls/Interrupts
    let kernel_stack = alloc::vec::Vec::<u8>::with_capacity(16384);
    let kernel_stack_top = kernel_stack.as_ptr() as u64 + 16384;
    arch::gdt::set_kernel_stack(kernel_stack_top);
    arch::syscall::set_syscall_kernel_stack(kernel_stack_top);
    core::mem::forget(kernel_stack);

    println!("[VAIN] Spawning kernel threads...");

    // Spawn Thread A
    let stack_a = alloc::vec::Vec::<u8>::with_capacity(16384);
    let stack_top_a = stack_a.as_ptr() as u64 + 16384;
    core::mem::forget(stack_a);
    sched::spawn_kernel_thread(10, thread_a, stack_top_a);

    // Spawn Thread B
    let stack_b = alloc::vec::Vec::<u8>::with_capacity(16384);
    let stack_top_b = stack_b.as_ptr() as u64 + 16384;
    core::mem::forget(stack_b);
    sched::spawn_kernel_thread(10, thread_b, stack_top_b);

    println!("[VAIN] Starting scheduler...");
    sched::schedule();

    unreachable!("Scheduler should not return");
}

extern "C" fn thread_a() -> ! {
    loop {
        crate::print!("A");
        // Waste time to slow down the loop
        for _ in 0..10_000_000 {
            core::hint::spin_loop();
        }
    }
}

extern "C" fn thread_b() -> ! {
    loop {
        crate::print!("B");
        // Waste time to slow down the loop
        for _ in 0..10_000_000 {
            core::hint::spin_loop();
        }
    }
}

#[panic_handler]
fn on_panic(panic_info: &core::panic::PanicInfo) -> ! {
    let mut serial = console::serial::SerialPort::COM1;
    let _ = writeln!(serial, "[VAIN PANIC] {}", panic_info);
    arch::halt_loop()
}
