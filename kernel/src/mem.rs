use bitflags::bitflags;

use crate::alloc::{self, PAGE_SIZE};
use crate::utils::is_aligned;

pub const SATP_SV39: usize = 8 << 60;
const VPN_MASK: usize = 0b1_1111_1111;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct PageFlags: usize {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

pub fn map_page(
    table2: &mut [usize],
    vaddr: usize,
    paddr: usize,
    flags: PageFlags,
    alloc: &mut alloc::Allocator,
) {
    if !is_aligned(vaddr, PAGE_SIZE) {
        panic!("vaddr={:p} is not aligned", vaddr as *const u8);
    }

    if !is_aligned(paddr, PAGE_SIZE) {
        panic!("paddr={:p} is not aligned", paddr as *const u8);
    }

    let vpn2 = vaddr >> 30 & VPN_MASK;
    if table2[vpn2] & PageFlags::V.bits() == 0 {
        // このエントリに対応する2段目のページテーブルが存在しないので作成する
        let pt_paddr = alloc.alloc_pages::<usize>(1).as_mut_ptr() as usize;
        table2[vpn2] = (pt_paddr / PAGE_SIZE) << 10 | PageFlags::V.bits();
    }

    let vpn1 = vaddr >> 21 & VPN_MASK;
    let table1 = unsafe {
        // table1のアドレスは [usize; 512] を指しているので *mut usize
        // 先ほど確保した pt_paddr を使わない理由は, 最初から定義されている場合もあるため
        let table1_addr = (table2[vpn2] >> 10) * PAGE_SIZE;

        if !is_aligned(table1_addr, PAGE_SIZE) {
            panic!("table1_addr={:p} is not aligned", table1_addr as *const u8);
        }

        core::slice::from_raw_parts_mut(table1_addr as *mut usize, 512)
    };
    if table1[vpn1] & PageFlags::V.bits() == 0 {
        // このエントリに対応する1段目のページテーブルが存在しないので作成する
        let pt_paddr = alloc.alloc_pages::<usize>(1).as_mut_ptr() as usize;
        table1[vpn1] = (pt_paddr / PAGE_SIZE) << 10 | PageFlags::V.bits();
    }

    let vpn0 = vaddr >> 12 & VPN_MASK;
    let table0 = unsafe {
        let table0_addr = (table1[vpn1] >> 10) * PAGE_SIZE;

        if !is_aligned(table0_addr, PAGE_SIZE) {
            panic!("table0_addr={:p} is not aligned", table0_addr as *const u8);
        }

        core::slice::from_raw_parts_mut(table0_addr as *mut usize, 512)
    };
    // TODO: A/Dビットの設定
    // ハードウェアの実装に依存する
    table0[vpn0] = (paddr / PAGE_SIZE) << 10 | flags.bits() | PageFlags::V.bits();
}
