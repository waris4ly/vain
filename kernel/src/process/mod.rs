use crate::arch::paging::PageTableEntry;
use crate::boot;
use crate::mm::frame_alloc;
use crate::mm::vmem;
use vain_elf::{ElfParser, PT_LOAD};

pub fn spawn_process(module_name: &str, capabilities: alloc::vec::Vec<crate::cap::Capability>) {
    let modules_response = boot::MODULES
        .get_response()
        .expect("No modules provided by bootloader");

    let mut found_module = None;
    for module in modules_response.modules() {
        let path = module.path().to_str().unwrap_or("");
        if path.contains(module_name) {
            found_module = Some(module);
            break;
        }
    }

    let module = found_module.unwrap_or_else(|| panic!("Module {} not found", module_name));
    let module_data = unsafe { core::slice::from_raw_parts(module.addr(), module.size() as usize) };
    let parser = ElfParser::new(module_data)
        .unwrap_or_else(|| panic!("Module {} is not a valid ELF", module_name));

    let process_cr3 = vmem::new_userspace_page_table().expect("Failed to create page table");

    let old_cr3: u64;
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) old_cr3, options(nomem, nostack, preserves_flags));
        vmem::switch_page_table(process_cr3);
    }

    for phdr in parser.program_headers() {
        if phdr.p_type == PT_LOAD {
            let vaddr = phdr.p_vaddr;
            let memsz = phdr.p_memsz;
            let filesz = phdr.p_filesz;
            let offset = phdr.p_offset;

            if vaddr < 0x1000 {
                panic!("Attempt to load ELF at invalid address: {:#x}", vaddr);
            }

            if offset as usize > module_data.len() {
                panic!("ELF offset out of bounds");
            }

            if (offset as usize).saturating_add(filesz as usize) > module_data.len() {
                panic!("ELF file size exceeds module data");
            }

            crate::println!(
                "PT_LOAD: vaddr={:#x}, offset={:#x}, filesz={:#x}, memsz={:#x}",
                vaddr,
                offset,
                filesz,
                memsz
            );

            let start_page = vaddr & !0xFFF;
            let end_page = match vaddr.checked_add(memsz) {
                Some(end) => (end + 0xFFF) & !0xFFF,
                None => panic!("ELF segment address overflow"),
            };

            if end_page >= 0x0000_8000_0000_0000 {
                panic!("ELF segment extends into kernel space");
            }

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

            if filesz > 0 {
                unsafe {
                    let dest = vaddr as *mut u8;
                    let src = module_data.as_ptr().add(offset as usize);
                    core::ptr::copy_nonoverlapping(src, dest, filesz as usize);
                }
            }

            if memsz > filesz {
                unsafe {
                    let dest = vaddr as *mut u8;
                    let bss = dest.add(filesz as usize);
                    core::ptr::write_bytes(bss, 0, (memsz - filesz) as usize);
                }
            }
        }
    }

    let user_stack_bottom = 0x700000000000u64;
    let user_stack_size = 16384u64;
    let user_stack_top = user_stack_bottom
        .checked_add(user_stack_size)
        .expect("User stack address overflow");

    for page_addr in (user_stack_bottom..user_stack_top).step_by(4096) {
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

    let entry_point = parser.header().e_entry;

    let heap_bottom = 0x40000000u64;
    let heap_size = 2 * 1024 * 1024u64;
    let heap_end = heap_bottom
        .checked_add(heap_size)
        .expect("User heap address overflow");

    for page_addr in (heap_bottom..heap_end).step_by(4096) {
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

    unsafe {
        vmem::switch_page_table(old_cr3);
    }

    crate::sched::spawn_userspace_thread(
        10,
        entry_point,
        user_stack_top,
        process_cr3,
        capabilities,
    );
}
