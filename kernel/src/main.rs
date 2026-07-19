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

    // Enable hardware interrupts
    arch::enable_interrupts();

    arch::syscall::init();

    // Allocate a kernel stack for Syscalls/Interrupts
    let kernel_stack = alloc::vec::Vec::<u8>::with_capacity(16384);
    let kernel_stack_top = kernel_stack.as_ptr() as u64 + 16384;
    arch::gdt::set_kernel_stack(kernel_stack_top);
    arch::syscall::set_syscall_kernel_stack(kernel_stack_top);
    core::mem::forget(kernel_stack);

    println!("[VAIN] Spawning kernel threads for IPC test...");
    
    // Create an Endpoint and store it globally for the test
    *TEST_ENDPOINT.lock() = Some(alloc::sync::Arc::new(ipc::endpoint::Endpoint::new()));
    
    // Spawn IPC Client (Thread A)
    let stack_a = alloc::vec::Vec::<u8>::with_capacity(16384);
    let stack_top_a = stack_a.as_ptr() as u64 + 16384;
    core::mem::forget(stack_a);
    sched::spawn_kernel_thread(10, ipc_client_thread, stack_top_a);
    
    // Spawn IPC Server (Thread B)
    let stack_b = alloc::vec::Vec::<u8>::with_capacity(16384);
    let stack_top_b = stack_b.as_ptr() as u64 + 16384;
    core::mem::forget(stack_b);
    sched::spawn_kernel_thread(10, ipc_server_thread, stack_top_b);

    println!("[VAIN] Starting scheduler...");
    sched::schedule();

    unreachable!("Scheduler should not return");
}

static TEST_ENDPOINT: crate::sync::Spinlock<Option<alloc::sync::Arc<ipc::endpoint::Endpoint>>> = crate::sync::Spinlock::new(None);

extern "C" fn ipc_client_thread() -> ! {
    let endpoint = TEST_ENDPOINT.lock().as_ref().unwrap().clone();
    
    // Wait a bit to ensure server is ready
    for _ in 0..1_000_000 {
        core::hint::spin_loop();
    }
    
    crate::println!("[Client] Sending message...");
    
    // Set up message in own TCB
    {
        let mut lock = sched::CURRENT_THREAD.lock();
        let current = lock.as_mut().unwrap();
        current.ipc_buffer.tag = 42;
        current.ipc_buffer.data[0] = 0xDEADBEEF;
    }
    
    endpoint.send();
    
    // Wait for reply? We didn't implement 'call' yet, so let's just recv
    endpoint.recv();
    
    let reply = {
        let mut lock = sched::CURRENT_THREAD.lock();
        let current = lock.as_mut().unwrap();
        current.ipc_buffer.data[0]
    };
    
    crate::println!("[Client] Received reply: {:#x}", reply);
    
    loop {
        core::hint::spin_loop();
    }
}

extern "C" fn ipc_server_thread() -> ! {
    let endpoint = TEST_ENDPOINT.lock().as_ref().unwrap().clone();
    
    crate::println!("[Server] Waiting for message...");
    endpoint.recv();
    
    let msg_data = {
        let mut lock = sched::CURRENT_THREAD.lock();
        let current = lock.as_mut().unwrap();
        crate::println!("[Server] Received tag: {}, data: {:#x}", current.ipc_buffer.tag, current.ipc_buffer.data[0]);
        current.ipc_buffer.data[0]
    };
    
    // Send reply
    {
        let mut lock = sched::CURRENT_THREAD.lock();
        let current = lock.as_mut().unwrap();
        current.ipc_buffer.tag = 100;
        current.ipc_buffer.data[0] = msg_data + 1;
    }
    
    crate::println!("[Server] Sending reply...");
    endpoint.send();
    
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn on_panic(panic_info: &core::panic::PanicInfo) -> ! {
    let mut serial = console::serial::SerialPort::COM1;
    let _ = writeln!(serial, "[VAIN PANIC] {}", panic_info);
    arch::halt_loop()
}
