use crate::{println, vmem::PageFlags};
use bitflags::bitflags;
use zerocopy::{FromBytes, FromZeroes};

//
// ELFヘッダ, プログラムヘッダの構造体
//

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

//
// ELFヘッダのパースをする
// create_process_from_loaded() に渡せる形にする
//

const PT_LOAD: u32 = 1;
const SEGMENT_MAX: usize = 12;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct SegmentFlags: u32 {
        const X = 1 << 0;
        const W = 1 << 1;
        const R = 1 << 2;
    }
}

#[derive(Debug)]
pub struct LoadableSegment {
    pub flags: PageFlags,
    pub vaddr: usize,
    /// filesz と同じ長さを持つ
    pub data: &'static [u8],
    pub filesz: usize,
    pub memsz: usize,
}

#[derive(Debug)]
pub struct LoadedElf {
    pub entry_point: usize,
    pub loadable_segments: [Option<LoadableSegment>; SEGMENT_MAX],
}

pub fn load_elf(elf_data: &'static [u8]) -> LoadedElf {
    let ehdr = Elf64Ehdr::read_from_prefix(elf_data).unwrap();

    println!("Loading ELF header:");
    let e_entry = ehdr.e_entry as usize;
    let e_phoff = ehdr.e_phoff as usize;
    let e_phnum = ehdr.e_phnum as usize;
    println!("\te_entry={:#x}", e_entry);
    println!("\te_phoff={:#x}", e_phoff);
    println!("\te_phnum={:#x}", e_phnum);

    // TODO: 動的配列が作れるようになったら変える
    let mut segments = [const { None }; SEGMENT_MAX];

    // TODO: flagsの情報を利用したい
    for i in 0..e_phnum {
        // プログラムヘッダの情報が入った構造体を作る
        let ph_start = e_phoff + i * size_of::<Elf64Phdr>();
        let phdr =
            Elf64Phdr::read_from_prefix(&elf_data[ph_start..]).expect("read_from_prefix failed!");

        // PT_LOAD のみを収集する
        if phdr.p_type != PT_LOAD {
            continue;
        }

        println!("Program header (PT_LOAD) {}:", i);

        // 変数を取り出す
        let p_type = phdr.p_type;
        let p_flags = phdr.p_flags;
        let p_offset = phdr.p_offset as usize;
        let p_vaddr = phdr.p_vaddr as usize;
        let p_filesz = phdr.p_filesz as usize;
        let p_memsz = phdr.p_memsz as usize;
        println!("\tp_type={:#x}", p_type);
        println!("\tp_flags={:#x}", p_flags);
        println!("\tp_offset={:#x}", p_offset);
        println!("\tp_vaddr={:#x}", p_vaddr);
        println!("\tp_filesz={:#x}", p_filesz);
        println!("\tp_memsz={:#x}", p_memsz);

        // フラグをページテーブルの使うフラグに変換する
        let seg_flags = SegmentFlags::from_bits_truncate(p_flags);
        let mut page_flags = PageFlags::empty();
        if seg_flags.contains(SegmentFlags::R) {
            page_flags |= PageFlags::R;
        }
        if seg_flags.contains(SegmentFlags::W) {
            page_flags |= PageFlags::W;
        }
        if seg_flags.contains(SegmentFlags::X) {
            page_flags |= PageFlags::X;
        }

        let seg = LoadableSegment {
            flags: page_flags,
            vaddr: p_vaddr,
            data: &elf_data[p_offset..p_offset + p_filesz],
            filesz: p_filesz,
            memsz: p_memsz,
        };

        segments[i] = Some(seg);
    }

    LoadedElf {
        entry_point: e_entry,
        loadable_segments: segments,
    }
}
