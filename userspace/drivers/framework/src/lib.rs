#![no_std]

use libos::syscall;

pub trait Driver {
    fn init(&mut self) -> Result<(), &'static str>;
    fn handle_interrupt(&mut self);
}

pub fn run_driver<D: Driver>(mut driver: D, irq_handle: u64) -> ! {
    if let Err(e) = driver.init() {
        libos::println!("Driver init failed: {}", e);
        syscall::sys_exit(1);
    }

    libos::println!("Driver started successfully. Entering IRQ loop.");

    loop {
        syscall::sys_wait_irq(irq_handle);
        driver.handle_interrupt();
    }
}
