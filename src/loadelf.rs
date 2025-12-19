use crate::println;
use zerocopy::{FromBytes, FromZeroes};

#[derive(Debug, FromZeroes, FromBytes)]
#[repr(C)]
struct Elf64Ehdr {
    e_ident: [u8; 16], /* Magic number and other info */
    e_type: u16,       /* Object file type */
    e_machine: u16,    /* Architecture */
    e_version: u32,    /* Object file version */
    e_entry: u64,      /* Entry point virtual address */
    e_phoff: u64,      /* Program header table file offset */
    e_shoff: u64,      /* Section header table file offset */
    e_flags: u32,      /* Processor-specific flags */
    e_ehsize: u16,     /* ELF header size in bytes */
    e_phentsize: u16,  /* Program header table entry size */
    e_phnum: u16,      /* Program header table entry count */
    e_shentsize: u16,  /* Section header table entry size */
    e_shnum: u16,      /* Section header table entry count */
    e_shstrndx: u16,   /* Section header string table index */
}

#[derive(Debug, FromZeroes, FromBytes)]
#[repr(C)]
struct Elf64Phdr {
    p_type: u32,   /* Segment type */
    p_flags: u32,  /* Segment flags */
    p_offset: u64, /* Segment file offset */
    p_vaddr: u64,  /* Segment virtual address */
    p_paddr: u64,  /* Segment physical address */
    p_filesz: i64, /* Segment size in file */
    p_memsz: u64,  /* Segment size in memory */
    p_align: u64,  /* Segment alignment */
}

static SHELL_ELF: &[u8] = include_bytes!("../shell.elf");

pub fn test_read_elf() {
    println!("[test_read_elf] shell.elf at {:p}", SHELL_ELF.as_ptr());
    let ehdr = Elf64Ehdr::read_from_prefix(SHELL_ELF).unwrap();

    println!("ELF header:");
    let e_entry = ehdr.e_entry;
    let e_phoff = ehdr.e_phoff;
    let e_phnum = ehdr.e_phnum;
    println!("\te_entry={:#x}", e_entry);
    println!("\te_phoff={:#x}", e_phoff);
    println!("\te_phnum={:#x}", e_phnum);

    let mut phdr_ptr = &SHELL_ELF[e_phoff as usize..];

    for i in 0..e_phnum {
        let phdr = Elf64Phdr::ref_from_prefix(phdr_ptr).unwrap();

        if phdr.p_type != 0x1 {
            continue;
        }

        println!("Program header (PT_LOAD) {}:", i);
        let p_type = phdr.p_type;
        let p_flags = phdr.p_flags;
        let p_offset = phdr.p_offset;
        let p_vaddr = phdr.p_vaddr;
        let p_filesz = phdr.p_filesz;
        let p_memsz = phdr.p_memsz;
        println!("\tp_type={:#x}", p_type);
        println!("\tp_flags={:#x}", p_flags);
        println!("\tp_offset={:#x}", p_offset);
        println!("\tp_vaddr={:#x}", p_vaddr);
        println!("\tp_filesz={:#x}", p_filesz);
        println!("\tp_memsz={:#x}", p_memsz);

        phdr_ptr = &phdr_ptr[size_of::<Elf64Phdr>()..];
    }
}
