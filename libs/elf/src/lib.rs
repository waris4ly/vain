#![no_std]

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64_Ehdr {
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64_Phdr {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

pub const PT_LOAD: u32 = 1;

pub struct ElfParser<'a> {
    data: &'a [u8],
}

impl<'a> ElfParser<'a> {
    pub fn new(data: &'a [u8]) -> Option<Self> {
        if data.len() < core::mem::size_of::<Elf64_Ehdr>() {
            return None;
        }

        let header = unsafe { &*(data.as_ptr() as *const Elf64_Ehdr) };

        // Check magic: \x7F E L F
        if header.e_ident[0..4] != [0x7f, b'E', b'L', b'F'] {
            return None;
        }

        Some(Self { data })
    }

    pub fn header(&self) -> &Elf64_Ehdr {
        unsafe { &*(self.data.as_ptr() as *const Elf64_Ehdr) }
    }

    pub fn program_headers(&self) -> impl Iterator<Item = &Elf64_Phdr> {
        let header = self.header();
        let phoff = header.e_phoff as usize;
        let phnum = header.e_phnum as usize;
        let phentsize = header.e_phentsize as usize;

        (0..phnum).map(move |i| {
            let offset = phoff + i * phentsize;
            unsafe { &*(self.data.as_ptr().add(offset) as *const Elf64_Phdr) }
        })
    }
}
