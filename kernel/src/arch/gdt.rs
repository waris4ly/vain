use core::arch::asm;

const GDT_ENTRY_COUNT: usize = 7;

#[repr(C)]
struct GlobalDescriptorTable {
    entries: [u64; GDT_ENTRY_COUNT],
}

impl GlobalDescriptorTable {
    pub const KERNEL_CODE: u16 = 1;
    pub const KERNEL_DATA: u16 = 2;
    #[allow(dead_code)]
    pub const USER_DATA: u16 = 3;
    #[allow(dead_code)]
    pub const USER_CODE: u16 = 4;
    pub const TSS: u16 = 5; // Takes 2 slots (5 and 6)

    pub fn set_tss(&mut self, tss: *const TaskStateSegment) {
        let ptr = tss as u64;
        let mut low = (1 << 47) | (0b1001 << 40); // Present, 64-bit TSS (Available)
        let limit = (core::mem::size_of::<TaskStateSegment>() - 1) as u64;
        low |= (limit & 0xFFFF) | ((limit & 0xF0000) << 32);
        low |= ((ptr & 0xFFFFFF) << 16) | ((ptr & 0xFF000000) << 32);

        let high = ptr >> 32;

        self.entries[Self::TSS as usize] = low;
        self.entries[(Self::TSS as usize) + 1] = high;
    }

    pub fn load(&self) {
        unsafe {
            let descriptor = GdtDescriptor {
                size: (core::mem::size_of::<GlobalDescriptorTable>() - 1) as u16,
                offset: self as *const _ as u64,
            };
            asm!("lgdt [{}]", in(reg) &descriptor, options(readonly, nostack, preserves_flags));
        }
    }
}

#[repr(C, packed)]
struct GdtDescriptor {
    size: u16,
    offset: u64,
}

#[derive(Clone, Copy)]
pub struct SegmentSelector(pub u16);

pub const KERNEL_CODE_SELECTOR: SegmentSelector =
    SegmentSelector(GlobalDescriptorTable::KERNEL_CODE << 3);
pub const KERNEL_DATA_SELECTOR: SegmentSelector =
    SegmentSelector(GlobalDescriptorTable::KERNEL_DATA << 3);

static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable {
    entries: [
        0, // Null descriptor
        // Kernel Code: Executable, Readable, Present, 64-bit
        (1 << 43) | (1 << 44) | (1 << 47) | (1 << 53),
        // Kernel Data: Writable, Present
        (1 << 41) | (1 << 44) | (1 << 47),
        // User Data: Writable, Present, DPL 3
        (1 << 41) | (1 << 44) | (1 << 45) | (1 << 46) | (1 << 47),
        // User Code: Executable, Readable, Present, 64-bit, DPL 3
        (1 << 43) | (1 << 44) | (1 << 45) | (1 << 46) | (1 << 47) | (1 << 53),
        0,
        0,
    ],
};

#[repr(C, packed)]
pub struct TaskStateSegment {
    reserved_1: u32,
    pub rsp0: u64,
    pub rsp1: u64,
    pub rsp2: u64,
    reserved_2: u64,
    pub ist: [u64; 7],
    reserved_3: u64,
    reserved_4: u16,
    pub iomap_base: u16,
}

impl TaskStateSegment {
    pub const fn new() -> Self {
        Self {
            reserved_1: 0,
            rsp0: 0,
            rsp1: 0,
            rsp2: 0,
            reserved_2: 0,
            ist: [0; 7],
            reserved_3: 0,
            reserved_4: 0,
            iomap_base: core::mem::size_of::<TaskStateSegment>() as u16,
        }
    }
}

static mut TSS: TaskStateSegment = TaskStateSegment::new();

pub fn init() {
    unsafe {
        (*core::ptr::addr_of_mut!(GDT)).set_tss(core::ptr::addr_of!(TSS));
        (*core::ptr::addr_of!(GDT)).load();

        // Load TSS
        asm!("ltr ax", in("ax") (GlobalDescriptorTable::TSS << 3), options(nostack, preserves_flags));

        // Reload segments
        asm!(
            "push {0}",
            "lea {0}, [2f]",
            "push {0}",
            "retfq",
            "2:",
            "mov ds, {1:x}",
            "mov es, {1:x}",
            "mov fs, {1:x}",
            "mov gs, {1:x}",
            "mov ss, {1:x}",
            in(reg) KERNEL_CODE_SELECTOR.0 as u64,
            in(reg) KERNEL_DATA_SELECTOR.0,
            options(nomem, nostack)
        );
    }
}

pub fn set_kernel_stack(stack_top: u64) {
    unsafe {
        (*core::ptr::addr_of_mut!(TSS)).rsp0 = stack_top;
    }
}
