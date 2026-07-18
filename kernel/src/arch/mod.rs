pub mod acpi;
pub mod apic;
pub mod gdt;
pub mod idt;
pub mod paging;
pub mod syscall;

pub fn init() {
    gdt::init();
    idt::init();
}

pub fn halt_loop() -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

pub fn enable_interrupts() {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack));
    }
}
