use crate::arch::paging::{PageTable, PageTableEntry, flush_tlb};
use crate::boot;
use crate::mm::frame_alloc;
use core::arch::asm;

pub fn init() {
    // The bootloader has already set up a 4-level page table for us, including a higher half direct map.
    // For this phase, we just rely on it, but we prepare the structures for future modifications.
}

pub unsafe fn active_level_4_table() -> &'static mut PageTable {
    let cr3: u64;
    unsafe { asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack, preserves_flags)) };

    // The physical address of the level 4 table is in cr3 (masking out flags)
    let phys = cr3 & 0x000F_FFFF_FFFF_F000;
    let virt = phys + boot::hhdm_offset();
    unsafe { &mut *(virt as *mut PageTable) }
}

pub unsafe fn map_page(
    virtual_address: u64,
    physical_address: u64,
    flags: u64,
) -> Result<(), &'static str> {
    let p4 = unsafe { active_level_4_table() };
    let hhdm = boot::hhdm_offset();

    let p4_idx = ((virtual_address >> 39) & 0x1FF) as usize;
    let p3_idx = ((virtual_address >> 30) & 0x1FF) as usize;
    let p2_idx = ((virtual_address >> 21) & 0x1FF) as usize;
    let p1_idx = ((virtual_address >> 12) & 0x1FF) as usize;

    let p3 = unsafe { get_or_create_table(&mut p4.entries[p4_idx], hhdm)? };
    let p2 = unsafe { get_or_create_table(&mut p3.entries[p3_idx], hhdm)? };
    let p1 = unsafe { get_or_create_table(&mut p2.entries[p2_idx], hhdm)? };

    let entry = &mut p1.entries[p1_idx];
    if entry.is_present() {
        return Err("Page already mapped");
    }

    entry.set_address(physical_address, flags | PageTableEntry::PRESENT);
    flush_tlb(virtual_address);
    Ok(())
}

unsafe fn get_or_create_table(
    entry: &mut PageTableEntry,
    hhdm: u64,
) -> Result<&'static mut PageTable, &'static str> {
    if !entry.is_present() {
        let frame = frame_alloc::alloc_frame().ok_or("Out of memory allocating page table")?;

        let virt = frame + hhdm;
        let table = unsafe { &mut *(virt as *mut PageTable) };
        table.clear();

        entry.set_address(
            frame,
            PageTableEntry::PRESENT | PageTableEntry::WRITABLE | PageTableEntry::USER_ACCESSIBLE,
        );
    }

    let virt = entry.address() + hhdm;
    Ok(unsafe { &mut *(virt as *mut PageTable) })
}

pub unsafe fn is_mapped(virtual_address: u64) -> bool {
    let p4 = unsafe { active_level_4_table() };
    let hhdm = boot::hhdm_offset();

    let p4_idx = ((virtual_address >> 39) & 0x1FF) as usize;
    let p3_idx = ((virtual_address >> 30) & 0x1FF) as usize;
    let p2_idx = ((virtual_address >> 21) & 0x1FF) as usize;
    let p1_idx = ((virtual_address >> 12) & 0x1FF) as usize;

    let p4_entry = &p4.entries[p4_idx];
    if !p4_entry.is_present() {
        return false;
    }

    let p3 = unsafe { &*((p4_entry.address() + hhdm) as *const PageTable) };
    let p3_entry = &p3.entries[p3_idx];
    if !p3_entry.is_present() {
        return false;
    }

    let p2 = unsafe { &*((p3_entry.address() + hhdm) as *const PageTable) };
    let p2_entry = &p2.entries[p2_idx];
    if !p2_entry.is_present() {
        return false;
    }

    let p1 = unsafe { &*((p2_entry.address() + hhdm) as *const PageTable) };
    let p1_entry = &p1.entries[p1_idx];
    p1_entry.is_present()
}
