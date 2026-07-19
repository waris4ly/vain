use crate::arch::paging::PageTableEntry;
use crate::boot;
use crate::mm::frame_alloc;
use crate::mm::vmem;
use vain_elf::{ElfParser, PT_LOAD};

pub fn load_init() {
    let modules_response = boot::MODULES
        .get_response()
        .expect("No modules provided by bootloader");

    let mut init_module = None;
    for module in modules_response.modules() {
        // Find the "init" module. For now, we'll just take the first one or one with "init" in the path.
        init_module = Some(module);
        break;
    }

    let module = init_module.expect("Init module not found");

    // Limine file struct provides addr() and size() in newer versions, or base and length.
    // We will use standard slice conversion.
    let module_data = unsafe { core::slice::from_raw_parts(module.addr(), module.size() as usize) };

    let parser = ElfParser::new(module_data).expect("Init module is not a valid ELF");

    // Map segments
    for phdr in parser.program_headers() {
        if phdr.p_type == PT_LOAD {
            let vaddr = phdr.p_vaddr;
            let memsz = phdr.p_memsz;
            let filesz = phdr.p_filesz;
            let offset = phdr.p_offset;

            crate::println!(
                "PT_LOAD: vaddr={:#x}, offset={:#x}, filesz={:#x}, memsz={:#x}",
                vaddr,
                offset,
                filesz,
                memsz
            );

            // Page align everything
            let start_page = vaddr & !0xFFF;
            let end_page = (vaddr + memsz + 0xFFF) & !0xFFF;

            for page_addr in (start_page..end_page).step_by(4096) {
                unsafe {
                    if !vmem::is_mapped(page_addr) {
                        let frame = frame_alloc::alloc_frame().expect("Out of memory loading init");
                        let flags = PageTableEntry::PRESENT
                            | PageTableEntry::WRITABLE
                            | PageTableEntry::USER_ACCESSIBLE;
                        vmem::map_page(page_addr, frame, flags).expect("Failed to map init page");
                    }
                }
            }

            // Copy data if there is any
            if filesz > 0 {
                unsafe {
                    let dest = vaddr as *mut u8;
                    let src = module_data.as_ptr().add(offset as usize);
                    core::ptr::copy_nonoverlapping(src, dest, filesz as usize);
                }
            }

            // Zero out the remaining BSS section
            if memsz > filesz {
                unsafe {
                    let dest = vaddr as *mut u8;
                    let bss = dest.add(filesz as usize);
                    core::ptr::write_bytes(bss, 0, (memsz - filesz) as usize);
                }
            }
        }
    }

    // Allocate a 16KB user stack at a fixed address
    let user_stack_bottom = 0x700000000000u64;
    let user_stack_size = 16384u64;
    for page_addr in (user_stack_bottom..user_stack_bottom + user_stack_size).step_by(4096) {
        unsafe {
            if !vmem::is_mapped(page_addr) {
                let frame = frame_alloc::alloc_frame().expect("Out of memory for user stack");
                let flags = PageTableEntry::PRESENT
                    | PageTableEntry::WRITABLE
                    | PageTableEntry::USER_ACCESSIBLE;
                vmem::map_page(page_addr, frame, flags).expect("Failed to map user stack");
            }
        }
    }
    
    let user_stack_top = user_stack_bottom + user_stack_size;
    let entry_point = parser.header().e_entry;

    // Allocate 2MB heap at 0x40000000 (just like libos expects)
    let heap_bottom = 0x40000000u64;
    let heap_size = 2 * 1024 * 1024u64;
    for page_addr in (heap_bottom..heap_bottom + heap_size).step_by(4096) {
        unsafe {
            if !vmem::is_mapped(page_addr) {
                let frame = frame_alloc::alloc_frame().expect("Out of memory for user heap");
                let flags = PageTableEntry::PRESENT
                    | PageTableEntry::WRITABLE
                    | PageTableEntry::USER_ACCESSIBLE;
                vmem::map_page(page_addr, frame, flags).expect("Failed to map user heap");
            }
        }
    }

    crate::sched::spawn_userspace_thread(10, entry_point, user_stack_top);
}
