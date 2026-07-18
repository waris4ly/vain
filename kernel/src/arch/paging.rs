#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    pub const PRESENT: u64 = 1 << 0;
    pub const WRITABLE: u64 = 1 << 1;
    pub const USER_ACCESSIBLE: u64 = 1 << 2;
    pub const WRITE_THROUGH: u64 = 1 << 3;
    pub const NO_CACHE: u64 = 1 << 4;
    pub const ACCESSED: u64 = 1 << 5;
    pub const DIRTY: u64 = 1 << 6;
    pub const HUGE_PAGE: u64 = 1 << 7;
    pub const GLOBAL: u64 = 1 << 8;
    pub const NO_EXECUTE: u64 = 1 << 63;

    const ADDRESS_MASK: u64 = 0x000F_FFFF_FFFF_F000;

    pub fn new() -> Self {
        Self(0)
    }

    pub fn set_address(&mut self, physical_address: u64, flags: u64) {
        self.0 = (physical_address & Self::ADDRESS_MASK) | flags;
    }

    pub fn address(&self) -> u64 {
        self.0 & Self::ADDRESS_MASK
    }

    pub fn flags(&self) -> u64 {
        self.0 & !Self::ADDRESS_MASK
    }

    pub fn is_present(&self) -> bool {
        (self.0 & Self::PRESENT) != 0
    }

    pub fn set_unused(&mut self) {
        self.0 = 0;
    }
}

#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [PageTableEntry; 512],
}

impl PageTable {
    pub fn new() -> Self {
        Self {
            entries: [PageTableEntry::new(); 512],
        }
    }

    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.set_unused();
        }
    }
}

pub fn flush_tlb(virtual_address: u64) {
    unsafe {
        core::arch::asm!("invlpg [{}]", in(reg) virtual_address, options(nostack, preserves_flags));
    }
}
