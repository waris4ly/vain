use crate::arch::acpi;
use crate::boot;
use core::arch::asm;

const PIC1_DATA: u16 = 0x21;
const PIC2_DATA: u16 = 0xA1;

#[allow(dead_code)]
unsafe fn outb(port: u16, val: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") val,
            options(nomem, nostack, preserves_flags)
        );
    }
}

#[allow(dead_code)]
unsafe fn inb(port: u16) -> u8 {
    let val: u8;
    unsafe {
        asm!(
            "in al, dx",
            out("al") val,
            in("dx") port,
            options(nomem, nostack, preserves_flags)
        );
    }
    val
}

#[allow(dead_code)]
const LAPIC_ID: u32 = 0x020;
const LAPIC_EOI: u32 = 0x0B0;
const LAPIC_SPURIOUS: u32 = 0x0F0;
const LAPIC_LVT_TIMER: u32 = 0x320;
const LAPIC_TIMER_INITCNT: u32 = 0x380;
#[allow(dead_code)]
const LAPIC_TIMER_CURCNT: u32 = 0x390;
const LAPIC_TIMER_DIV: u32 = 0x3E0;

// IOAPIC Registers
const IOAPIC_REG_ID: u32 = 0x00;
const IOAPIC_REG_VER: u32 = 0x01;
const IOAPIC_REG_TABLE: u32 = 0x10;

static mut LAPIC_VIRT: u64 = 0;
static mut IOAPIC_VIRT: u64 = 0;

#[allow(dead_code)]
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
        outb(PIC1_DATA, 0xFF);
        outb(PIC2_DATA, 0xFF);
        crate::println!("[VAIN APIC] Disabled Legacy PIC");

        let acpi_info = acpi::init();

        let hhdm = boot::hhdm_offset();
        let lapic_phys = acpi_info.lapic_base;
        let lapic_page = lapic_phys & !0xFFF;
        let _ = crate::mm::vmem::map_page(
            lapic_page + hhdm,
            lapic_page,
            crate::arch::paging::PageTableEntry::PRESENT
                | crate::arch::paging::PageTableEntry::WRITABLE
                | crate::arch::paging::PageTableEntry::NO_CACHE,
        );

        LAPIC_VIRT = lapic_phys + hhdm;

        write_lapic(LAPIC_SPURIOUS, 0x100 | 0xFF);

        write_lapic(LAPIC_TIMER_DIV, 0x3);
        write_lapic(LAPIC_LVT_TIMER, (1 << 17) | 32);
        write_lapic(LAPIC_TIMER_INITCNT, 10000000);

        crate::println!("[VAIN APIC] Local APIC and Timer Initialized");

        if let Some(ioapic_phys) = acpi_info.ioapic_base {
            let ioapic_page = ioapic_phys & !0xFFF;
            let _ = crate::mm::vmem::map_page(
                ioapic_page + hhdm,
                ioapic_page,
                crate::arch::paging::PageTableEntry::PRESENT
                    | crate::arch::paging::PageTableEntry::WRITABLE
                    | crate::arch::paging::PageTableEntry::NO_CACHE,
            );

            IOAPIC_VIRT = ioapic_phys + hhdm;

            let ioapic_id = (read_ioapic(IOAPIC_REG_ID) >> 24) & 0xF;
            let ioapic_ver = read_ioapic(IOAPIC_REG_VER);
            let max_intr = (ioapic_ver >> 16) & 0xFF;

            crate::println!(
                "[VAIN APIC] IOAPIC ID: {}, Max Intr: {}",
                ioapic_id,
                max_intr
            );

            let irq = 1;
            let vector = 33;
            let lapic_id_target = 0;

            let low_val = vector | (0 << 8) | (0 << 11) | (0 << 13) | (0 << 15);
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
