use crate::boot;

#[repr(C, packed)]
struct RsdpDescriptor {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
}

#[repr(C, packed)]
struct RsdpDescriptor20 {
    first_part: RsdpDescriptor,
    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    reserved: [u8; 3],
}

#[repr(C, packed)]
struct SdtHeader {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

#[repr(C, packed)]
struct MadtHeader {
    header: SdtHeader,
    local_apic_address: u32,
    flags: u32,
}

#[derive(Debug)]
pub struct AcpiInfo {
    pub lapic_base: u64,
    pub ioapic_base: Option<u64>,
}

pub fn init() -> AcpiInfo {
    let rsdp_addr = boot::RSDP
        .get_response()
        .expect("No RSDP provided by bootloader")
        .address();

    let hhdm = boot::hhdm_offset();

    // Limine provides the RSDP address. Ensure it is in the higher half.
    let mut rsdp_addr_virt = rsdp_addr as *const u8 as u64;
    if rsdp_addr_virt < hhdm {
        rsdp_addr_virt += hhdm;
    }

    // Ensure the page containing RSDP is mapped!
    // We map a few pages just in case the tables cross page boundaries.
    let rsdp_phys = rsdp_addr_virt - hhdm;
    let rsdp_page = rsdp_phys & !0xFFF;
    unsafe {
        for i in 0..8 {
            // map 8 pages (32KB) starting from the RSDP page
            let _ = crate::mm::vmem::map_page(
                rsdp_page + (i * 0x1000) + hhdm,
                rsdp_page + (i * 0x1000),
                crate::arch::paging::PageTableEntry::PRESENT
                    | crate::arch::paging::PageTableEntry::WRITABLE,
            );
        }
    }

    let rsdp = unsafe { &*(rsdp_addr_virt as *const RsdpDescriptor) };

    crate::println!(
        "[VAIN ACPI] RSDP Signature: {:?}",
        core::str::from_utf8(&rsdp.signature).unwrap_or("INVALID")
    );

    let mut xsdt_addr = 0;
    let mut is_xsdt = false;

    if rsdp.revision >= 2 {
        let rsdp20 = unsafe { &*(rsdp_addr_virt as *const RsdpDescriptor20) };
        xsdt_addr = rsdp20.xsdt_address as u64;
        is_xsdt = true;
    }

    let rsdt_addr = rsdp.rsdt_address as u64;

    crate::println!("[VAIN ACPI] rsdt_addr: {:#x}, hhdm: {:#x}", rsdt_addr, hhdm);

    let sdt_virt = if is_xsdt {
        // Physical address to HHDM
        xsdt_addr + hhdm
    } else {
        rsdt_addr + hhdm
    };

    // Map SDT page
    let sdt_phys = sdt_virt - hhdm;
    let sdt_page = sdt_phys & !0xFFF;
    unsafe {
        for i in 0..8 {
            let _ = crate::mm::vmem::map_page(
                sdt_page + (i * 0x1000) + hhdm,
                sdt_page + (i * 0x1000),
                crate::arch::paging::PageTableEntry::PRESENT
                    | crate::arch::paging::PageTableEntry::WRITABLE,
            );
        }
    }

    let sdt_header = unsafe { &*(sdt_virt as *const SdtHeader) };
    let entries_len = sdt_header.length as usize - core::mem::size_of::<SdtHeader>();

    let mut madt_virt = None;

    if is_xsdt {
        let entries_count = entries_len / 8;
        let entries_base = (sdt_virt + core::mem::size_of::<SdtHeader>() as u64) as *const u64;
        for i in 0..entries_count {
            let entry_phys = unsafe { core::ptr::read_unaligned(entries_base.add(i)) };
            let entry_virt = entry_phys + hhdm;

            // Map the table page
            let entry_page = entry_phys & !0xFFF;
            unsafe {
                let _ = crate::mm::vmem::map_page(
                    entry_page + hhdm,
                    entry_page,
                    crate::arch::paging::PageTableEntry::PRESENT
                        | crate::arch::paging::PageTableEntry::WRITABLE,
                );
            }

            let header = unsafe { &*(entry_virt as *const SdtHeader) };
            if &header.signature == b"APIC" {
                madt_virt = Some(entry_virt);
                break;
            }
        }
    } else {
        let entries_count = entries_len / 4;
        let entries_base = (sdt_virt + core::mem::size_of::<SdtHeader>() as u64) as *const u32;
        for i in 0..entries_count {
            let entry_phys = unsafe { core::ptr::read_unaligned(entries_base.add(i)) };
            let entry_virt = entry_phys as u64 + hhdm;

            // Map the table page
            let entry_page = (entry_phys as u64) & !0xFFF;
            unsafe {
                let _ = crate::mm::vmem::map_page(
                    entry_page + hhdm,
                    entry_page,
                    crate::arch::paging::PageTableEntry::PRESENT
                        | crate::arch::paging::PageTableEntry::WRITABLE,
                );
            }

            let header = unsafe { &*(entry_virt as *const SdtHeader) };
            if &header.signature == b"APIC" {
                madt_virt = Some(entry_virt);
                break;
            }
        }
    }

    let madt_virt = madt_virt.expect("MADT not found in ACPI tables");

    // Map MADT page
    let madt_phys = madt_virt - hhdm;
    let madt_page = madt_phys & !0xFFF;
    unsafe {
        for i in 0..4 {
            let _ = crate::mm::vmem::map_page(
                madt_page + (i * 0x1000) + hhdm,
                madt_page + (i * 0x1000),
                crate::arch::paging::PageTableEntry::PRESENT
                    | crate::arch::paging::PageTableEntry::WRITABLE,
            );
        }
    }

    let madt = unsafe { &*(madt_virt as *const MadtHeader) };

    let mut lapic_base = madt.local_apic_address as u64;
    let mut ioapic_base = None;

    let mut current_offset = core::mem::size_of::<MadtHeader>() as u64;
    let end_offset = madt.header.length as u64;

    while current_offset < end_offset {
        let entry_ptr = (madt_virt + current_offset) as *const u8;
        let entry_type = unsafe { *entry_ptr };
        let entry_length = unsafe { *entry_ptr.add(1) };

        match entry_type {
            1 => {
                // I/O APIC
                let ioapic_addr = unsafe { core::ptr::read_unaligned(entry_ptr.add(4) as *const u32) };
                if ioapic_base.is_none() {
                    ioapic_base = Some(ioapic_addr as u64);
                }
            }
            2 => {
                // Interrupt Source Override
                let bus = unsafe { *entry_ptr.add(2) };
                let source_irq = unsafe { *entry_ptr.add(3) };
                let global_sys_interrupt = unsafe { core::ptr::read_unaligned(entry_ptr.add(4) as *const u32) };
                let flags = unsafe { core::ptr::read_unaligned(entry_ptr.add(8) as *const u16) };
                crate::println!("[VAIN ACPI] Override: Bus {} IRQ {} -> GSI {} (Flags {:#x})", bus, source_irq, global_sys_interrupt, flags);
            }
            5 => {
                // 64-bit LAPIC Base Address Override
                let lapic_addr = unsafe { core::ptr::read_unaligned(entry_ptr.add(4) as *const u64) };
                lapic_base = lapic_addr;
            }
            _ => {}
        }

        current_offset += entry_length as u64;
    }

    crate::println!("[VAIN ACPI] Found LAPIC at {:#x}", lapic_base);
    if let Some(io) = ioapic_base {
        crate::println!("[VAIN ACPI] Found IOAPIC at {:#x}", io);
    }

    AcpiInfo {
        lapic_base,
        ioapic_base,
    }
}
