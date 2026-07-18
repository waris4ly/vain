use crate::arch::acpi;
use crate::arch::paging::PageTableEntry;
use crate::boot;
use crate::mm::vmem;
use core::arch::asm;

const PIC1_DATA: u16 = 0x21;
const PIC2_DATA: u16 = 0xA1;

unsafe fn outb(port: u16, val: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags))
    };
}

unsafe fn inb(port: u16) -> u8 {
    let mut val: u8;
    unsafe {
        asm!("in al, dx", out("al") val, in("dx") port, options(nomem, nostack, preserves_flags))
    };
    val
}

// Local APIC Registers
const LAPIC_ID: u32 = 0x020;
const LAPIC_EOI: u32 = 0x0B0;
const LAPIC_SPURIOUS: u32 = 0x0F0;
const LAPIC_LVT_TIMER: u32 = 0x320;
const LAPIC_TIMER_INITCNT: u32 = 0x380;
const LAPIC_TIMER_CURCNT: u32 = 0x390;
const LAPIC_TIMER_DIV: u32 = 0x3E0;

// IOAPIC Registers
const IOAPIC_REG_ID: u32 = 0x00;
const IOAPIC_REG_VER: u32 = 0x01;
const IOAPIC_REG_TABLE: u32 = 0x10;

static mut LAPIC_VIRT: u64 = 0;
static mut IOAPIC_VIRT: u64 = 0;

unsafe fn read_lapic(reg: u32) -> u32 {
    unsafe { core::ptr::read_volatile((LAPIC_VIRT + reg as u64) as *const u32) }
}

unsafe fn write_lapic(reg: u32, val: u32) {
    unsafe { core::ptr::write_volatile((LAPIC_VIRT + reg as u64) as *mut u32, val) };
}

unsafe fn read_ioapic(reg: u32) -> u32 {
    unsafe {
        core::ptr::write_volatile((IOAPIC_VIRT + 0x00) as *mut u32, reg);
        core::ptr::read_volatile((IOAPIC_VIRT + 0x10) as *const u32)
    }
}

unsafe fn write_ioapic(reg: u32, val: u32) {
    unsafe {
        core::ptr::write_volatile((IOAPIC_VIRT + 0x00) as *mut u32, reg);
        core::ptr::write_volatile((IOAPIC_VIRT + 0x10) as *mut u32, val);
    }
}

pub fn init() {
    unsafe {
        // 1. Disable legacy 8259 PIC
        outb(PIC1_DATA, 0xFF);
        outb(PIC2_DATA, 0xFF);
        crate::println!("[VAIN APIC] Disabled Legacy PIC");

        // 2. Parse ACPI to find APIC bases
        let acpi_info = acpi::init();

        // 3. Map LAPIC
        let hhdm = boot::hhdm_offset();
        let lapic_phys = acpi_info.lapic_base;
        // Map LAPIC page
        let lapic_page = lapic_phys & !0xFFF;
        unsafe {
            let _ = crate::mm::vmem::map_page(
                lapic_page + hhdm,
                lapic_page,
                crate::arch::paging::PageTableEntry::PRESENT
                    | crate::arch::paging::PageTableEntry::WRITABLE
                    | crate::arch::paging::PageTableEntry::NO_CACHE,
            );
        }

        unsafe { LAPIC_VIRT = lapic_phys + hhdm };

        // Enable LAPIC and set spurious interrupt vector to 0xFF
        write_lapic(LAPIC_SPURIOUS, 0x100 | 0xFF); // Bit 8 = APIC Software Enable

        // 4. Configure LAPIC Timer
        // Divider: divide by 16 (0x3)
        write_lapic(LAPIC_TIMER_DIV, 0x3);
        // LVT Timer: Periodic mode (bit 17) | Vector 32
        write_lapic(LAPIC_LVT_TIMER, (1 << 17) | 32);
        // Initial Count
        write_lapic(LAPIC_TIMER_INITCNT, 10000000); // Arbitrary tick value for now

        crate::println!("[VAIN APIC] Local APIC and Timer Initialized");

        // 5. Initialize IOAPIC
        if let Some(ioapic_phys) = acpi_info.ioapic_base {
            // Map IOAPIC page
            let ioapic_page = ioapic_phys & !0xFFF;
            unsafe {
                let _ = crate::mm::vmem::map_page(
                    ioapic_page + hhdm,
                    ioapic_page,
                    crate::arch::paging::PageTableEntry::PRESENT
                        | crate::arch::paging::PageTableEntry::WRITABLE
                        | crate::arch::paging::PageTableEntry::NO_CACHE,
                );
            }

            unsafe { IOAPIC_VIRT = ioapic_phys + hhdm };

            let ioapic_id = (read_ioapic(IOAPIC_REG_ID) >> 24) & 0xF;
            let ioapic_ver = read_ioapic(IOAPIC_REG_VER);
            let max_intr = (ioapic_ver >> 16) & 0xFF;

            crate::println!(
                "[VAIN APIC] IOAPIC ID: {}, Max Intr: {}",
                ioapic_id,
                max_intr
            );

            // Redirect IRQ 1 (PS/2 Keyboard) to Vector 33 on the BSP (LAPIC ID 0 for now)
            // IOAPIC Redirection Table Entry for IRQ 1 is at 0x10 + 2 * 1 = 0x12
            let irq = 1;
            let vector = 33;
            let lapic_id_target = 0; // Assuming our boot CPU is LAPIC ID 0

            let low_val = vector | (0 << 8) | (0 << 11) | (0 << 13) | (0 << 15); // Unmasked, Fixed delivery, Edge triggered
            let high_val = (lapic_id_target as u32) << 24;

            write_ioapic(IOAPIC_REG_TABLE + 2 * irq, low_val);
            write_ioapic(IOAPIC_REG_TABLE + 2 * irq + 1, high_val);

            crate::println!("[VAIN APIC] Redirected IRQ 1 to Vector {}", vector);
        } else {
            crate::println!("[VAIN APIC] WARNING: No IOAPIC found!");
        }
    }
}

pub fn end_of_interrupt() {
    unsafe {
        write_lapic(LAPIC_EOI, 0);
    }
}
