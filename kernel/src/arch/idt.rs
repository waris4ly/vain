use crate::console::serial::SerialPort;
use core::arch::asm;
use core::fmt::Write;

#[repr(C)]
#[derive(Debug)]
pub struct ExceptionStackFrame {
    instruction_pointer: u64,
    code_segment: u64,
    cpu_flags: u64,
    stack_pointer: u64,
    stack_segment: u64,
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct IdtEntry {
    pointer_low: u16,
    gdt_selector: u16,
    options: u16,
    pointer_middle: u16,
    pointer_high: u32,
    reserved: u32,
}

impl IdtEntry {
    const fn empty() -> Self {
        Self {
            pointer_low: 0,
            gdt_selector: 0,
            options: 0,
            pointer_middle: 0,
            pointer_high: 0,
            reserved: 0,
        }
    }

    fn set_handler(&mut self, handler: u64) {
        self.pointer_low = handler as u16;
        self.pointer_middle = (handler >> 16) as u16;
        self.pointer_high = (handler >> 32) as u32;
        self.gdt_selector = super::gdt::KERNEL_CODE_SELECTOR.0;
        self.options = 0x8E00; // Present | Interrupt Gate
    }
}

#[repr(C, align(16))]
struct InterruptDescriptorTable {
    entries: [IdtEntry; 256],
}

#[repr(C, packed)]
struct IdtDescriptor {
    size: u16,
    offset: u64,
}

static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable {
    entries: [IdtEntry::empty(); 256],
};

pub fn init() {
    unsafe {
        IDT.entries[0].set_handler(divide_by_zero_handler as *const () as u64);
        IDT.entries[8].set_handler(double_fault_handler as *const () as u64);
        IDT.entries[13].set_handler(general_protection_fault_handler as *const () as u64);
        IDT.entries[14].set_handler(page_fault_handler as *const () as u64);

        // Hardware interrupts
        IDT.entries[32].set_handler(timer_interrupt_handler as *const () as u64);
        IDT.entries[33].set_handler(keyboard_interrupt_handler as *const () as u64);

        let descriptor = IdtDescriptor {
            size: (core::mem::size_of::<InterruptDescriptorTable>() - 1) as u16,
            offset: core::ptr::addr_of!(IDT) as u64,
        };

        asm!("lidt [{}]", in(reg) &descriptor, options(readonly, nostack, preserves_flags));
    }
}

extern "x86-interrupt" fn divide_by_zero_handler(stack_frame: ExceptionStackFrame) {
    let mut serial = SerialPort::COM1;
    let _ = writeln!(
        serial,
        "[VAIN PANIC] EXCEPTION: DIVIDE BY ZERO\n{:#?}",
        stack_frame
    );
    crate::arch::halt_loop();
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: ExceptionStackFrame, _error_code: u64) {
    let mut serial = SerialPort::COM1;
    let _ = writeln!(
        serial,
        "[VAIN PANIC] EXCEPTION: DOUBLE FAULT\n{:#?}",
        stack_frame
    );
    crate::arch::halt_loop();
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: ExceptionStackFrame,
    error_code: u64,
) {
    let mut serial = SerialPort::COM1;
    let _ = writeln!(
        serial,
        "[VAIN PANIC] EXCEPTION: GENERAL PROTECTION FAULT (Code {})\n{:#?}",
        error_code, stack_frame
    );
    crate::arch::halt_loop();
}

extern "x86-interrupt" fn page_fault_handler(stack_frame: ExceptionStackFrame, error_code: u64) {
    let cr2: u64;
    unsafe { asm!("mov {}, cr2", out(reg) cr2, options(nomem, nostack, preserves_flags)) };

    let mut serial = SerialPort::COM1;
    let _ = writeln!(
        serial,
        "[VAIN PANIC] EXCEPTION: PAGE FAULT (Code {})\nAccessed Address: {:#x}\n{:#?}",
        error_code, cr2, stack_frame
    );
    crate::arch::halt_loop();
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: ExceptionStackFrame) {
    // crate::println!("[VAIN] Timer IRQ fired!");
    crate::arch::apic::end_of_interrupt();
    crate::sched::schedule();
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: ExceptionStackFrame) {
    crate::println!("[VAIN] Keyboard IRQ fired!");
    if let Some(notif) = crate::IRQ1_NOTIFICATION.lock().as_ref() {
        notif.signal();
    }
    crate::arch::apic::end_of_interrupt();
}
