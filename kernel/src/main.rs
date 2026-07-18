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

    println!("[VAIN] Loading userspace init...");
    let entry = process::load_init();

    // Allocate a user stack
    // Ideally we would map physical frames to a high user address like 0x80000000.
    // Let's just do it cleanly via page tables.
    let user_stack_top = 0x8000_0000;
    for page in 0..4 {
        // 16KB stack
        let frame = mm::frame_alloc::alloc_frame().unwrap();
        let flags = arch::paging::PageTableEntry::PRESENT
            | arch::paging::PageTableEntry::WRITABLE
            | arch::paging::PageTableEntry::USER_ACCESSIBLE;
        unsafe {
            mm::vmem::map_page(user_stack_top - (page + 1) * 4096, frame, flags).unwrap();
        }
    }

    println!("[VAIN] Transitioning to Ring 3 (entry: {:#x})...", entry);
    unsafe {
        arch::syscall::transition_to_user(entry, user_stack_top);
    }
}

#[panic_handler]
fn on_panic(panic_info: &core::panic::PanicInfo) -> ! {
    let mut serial = console::serial::SerialPort::COM1;
    let _ = writeln!(serial, "[VAIN PANIC] {}", panic_info);
    arch::halt_loop()
}
