use crate::boot;
use crate::sync::Spinlock;
use limine::memory_map::EntryType;
use vain_allocator::bitmap::BitmapAllocator;

const PAGE_SIZE: u64 = 4096;

static FRAME_ALLOCATOR: Spinlock<Option<BitmapAllocator<'static>>> = Spinlock::new(None);

pub fn init() {
    let mmap = boot::MEMORY_MAP.get_response().expect("No memory map");
    let hhdm = boot::hhdm_offset();

    let mut highest_addr = 0u64;
    for entry in mmap.entries() {
        let top = entry.base.saturating_add(entry.length);
        if top > highest_addr {
            highest_addr = top;
        }
    }

    if highest_addr == 0 {
        panic!("Invalid memory map: no memory regions found");
    }

    let total_frames = ((highest_addr / PAGE_SIZE) as usize).max(1);
    let bitmap_bytes = total_frames.div_ceil(8);

    let mut bitmap_phys_addr = 0;
    for entry in mmap.entries() {
        if entry.entry_type == EntryType::USABLE && entry.length >= bitmap_bytes as u64 {
            bitmap_phys_addr = entry.base;
            break;
        }
    }

    if bitmap_phys_addr == 0 {
        panic!("Could not find enough memory for frame allocator bitmap");
    }

    let bitmap_virt_addr = bitmap_phys_addr
        .checked_add(hhdm)
        .expect("Bitmap virtual address overflow");
    let bitmap_slice =
        unsafe { core::slice::from_raw_parts_mut(bitmap_virt_addr as *mut u8, bitmap_bytes) };

    let mut allocator = BitmapAllocator::new(bitmap_slice, total_frames);

    for entry in mmap.entries() {
        if entry.entry_type == EntryType::USABLE {
            let start_frame = (entry.base / PAGE_SIZE) as usize;
            let end_frame = ((entry.base.saturating_add(entry.length)) / PAGE_SIZE) as usize;

            for frame in start_frame..end_frame.min(total_frames) {
                allocator.mark_free(frame);
            }
        }
    }

    let bitmap_start_frame = (bitmap_phys_addr / PAGE_SIZE) as usize;
    let bitmap_end_frame =
        bitmap_start_frame.saturating_add(bitmap_bytes.div_ceil(PAGE_SIZE as usize));
    for frame in bitmap_start_frame..bitmap_end_frame.min(total_frames) {
        allocator.mark_used(frame);
    }

    *FRAME_ALLOCATOR.lock() = Some(allocator);
}

pub fn alloc_frame() -> Option<u64> {
    FRAME_ALLOCATOR
        .lock()
        .as_mut()
        .and_then(|a| a.alloc())
        .and_then(|f| (f as u64).checked_mul(PAGE_SIZE))
}

pub fn free_frame(phys_addr: u64) {
    if phys_addr & (PAGE_SIZE - 1) != 0 {
        return;
    }
    if let Some(a) = FRAME_ALLOCATOR.lock().as_mut() {
        let frame = (phys_addr / PAGE_SIZE) as usize;
        if frame < a.total_frames() {
            a.free(frame);
        }
    }
}
