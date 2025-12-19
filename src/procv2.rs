//
// プロセス管理構造体の定義
//

unsafe extern "C" {
    static __kernel_base: u8;
}

#[derive(Debug, PartialEq, Clone)]
struct Pid(usize);

impl Pid {
    #[inline]
    fn as_usize(&self) -> usize {
        self.0
    }
}

#[derive(Debug, PartialEq, Clone)]
enum ProcState {
    Unused,
    Runnable,
}

#[derive(Debug, Clone, PartialEq)]
struct KernelStack {
    base: *mut u8,
    size: usize,
}

impl KernelStack {
    /// topはスタックポインタで, 64bitレジスタの値を積むのでusizeのポインタとしている
    #[inline]
    fn top(&self) -> *mut usize {
        // add() は u8 の要素数で計算されている
        unsafe { self.base.add(self.size) as *mut usize }
    }

    const fn null() -> Self {
        Self {
            base: core::ptr::null_mut(),
            size: 0,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
#[repr(C)]
struct Context {
    // レジスタを順序通りに並べる
    ra: usize, // return address
    sp: usize, // stack pointer
    s0: usize,
    s1: usize,
    s2: usize,
    s3: usize,
    s4: usize,
    s5: usize,
    s6: usize,
    s7: usize,
    s8: usize,
    s9: usize,
    s10: usize,
    s11: usize,
}

impl Context {
    const fn zero() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s0: 0,
            s1: 0,
            s2: 0,
            s3: 0,
            s4: 0,
            s5: 0,
            s6: 0,
            s7: 0,
            s8: 0,
            s9: 0,
            s10: 0,
            s11: 0,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
struct Process {
    pid: Pid,
    state: ProcState,
    kernel_stack: KernelStack,
    context: Context,
    page_table: *mut [usize],
}

impl Process {
    const fn unused() -> Self {
        let null_ptr_slice = core::ptr::slice_from_raw_parts_mut(core::ptr::null_mut(), 0);
        Self {
            pid: Pid(usize::MAX),
            state: ProcState::Unused,
            kernel_stack: KernelStack::null(),
            context: Context::zero(),
            page_table: null_ptr_slice,
        }
    }
}

//
// プロセステーブルの定義
//

use core::arch::asm;
use core::{arch::naked_asm, cell::UnsafeCell};
use core::{slice, usize};

use crate::alloc::PAGE_SIZE;
use crate::utils::is_aligned;
use crate::vmem::{self, PageFlags};
use crate::{alloc, csr, loadelf, println};

struct ProcessTableCell<T> {
    inner: UnsafeCell<T>,
}

unsafe impl<T> Sync for ProcessTableCell<T> {}

impl<T> ProcessTableCell<T> {
    const fn new(value: T) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }

    #[inline]
    unsafe fn get(&self) -> &T {
        unsafe { &*self.inner.get() }
    }

    /// # Safety
    /// この参照のライフタイムは検証されない
    #[inline]
    unsafe fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.inner.get() }
    }
}

pub const NPROC: usize = 8;

struct ProcessTable {
    procs: [Process; NPROC],
    current: usize, // 実行中のプロセスへのインデックス
}

impl ProcessTable {
    const fn new() -> Self {
        Self {
            procs: [const { Process::unused() }; NPROC],
            current: 0,
        }
    }

    #[inline]
    fn procs_mut(&mut self) -> &mut [Process; NPROC] {
        &mut self.procs
    }

    #[inline]
    fn procs_ref(&self) -> &[Process; NPROC] {
        &self.procs
    }

    /// # Safety
    /// この呼び出し前に schedule() など, 内部のインデックスを変える操作を行っていないか
    #[inline]
    unsafe fn current_proc_ref(&self) -> &Process {
        &self.procs[self.current]
    }

    /// # Safety
    /// この呼び出し前に schedule() など, 内部のインデックスを変える操作を行っていないか
    #[inline]
    unsafe fn current_proc_mut_ref(&mut self) -> &mut Process {
        &mut self.procs[self.current]
    }
}

pub fn dump_process_list() {
    println!("[procv2] process list:");
    let ptable = unsafe { PTABLE.get() };
    for proc in ptable.procs.iter() {
        println!(
            "\tpid={}, state={:?}, ra={:p}, sp={:p}",
            proc.pid.as_usize(),
            proc.state,
            proc.context.ra as *const u8,
            proc.context.sp as *const u8,
        );
    }
}

static PTABLE: ProcessTableCell<ProcessTable> = ProcessTableCell::new(ProcessTable::new());

//
// プロセス管理
//

/// # Safety
/// プロセステーブルの指しているインデックスを変える関数
///
/// 切り替える次のプロセスは可変参照が不要なので参照を返している
fn schedule<'a>() -> &'a Process {
    let ptable = unsafe { PTABLE.get_mut() };
    let cur_idx = ptable.current;

    let procs = &ptable.procs;
    for i in 0..NPROC {
        let next_idx = (cur_idx + i + 1) % NPROC;
        let p = &procs[next_idx];
        if p.state == ProcState::Runnable && p.pid.as_usize() > 0 {
            // 実行可能かつ0ではないプロセスが見つかった場合
            // インデックスを更新してPidを返す
            ptable.current = next_idx;
            return p;
        }
    }
    return &procs[0];
}

fn yield_process() {
    let prev_proc = unsafe { PTABLE.get_mut().current_proc_mut_ref() };
    let next_proc = schedule();

    let prev_ctx = &mut prev_proc.context as *mut Context;
    let next_ctx = &next_proc.context as *const Context;
    println!("switching ... {:?} -> {:?}", prev_proc.pid, next_proc.pid);
    switch_context(prev_ctx, next_ctx);
}

pub fn create_process(elf_data: &'static [u8], allocator: &mut alloc::Allocator) {
    let loaded = loadelf::load_elf(elf_data);
    create_process_from_loaded(loaded, allocator);
}

fn create_process_from_loaded(loaded: loadelf::LoadedElf, allocator: &mut alloc::Allocator) {
    // プロセステーブルを &mut の参照で取得する
    // この参照のライフタイムは検証されないので, 複数つくらないようにする
    let procs = unsafe { PTABLE.get_mut().procs_mut() };

    // プロセステーブルの中で状態が Unused のうち最初に見つけたものを取得する
    let (idx, proc) = procs
        .iter_mut()
        .enumerate()
        .find(|(_, p)| p.state == ProcState::Unused)
        .expect("create process failed!");

    // カーネルスタック領域の取得
    let page_count = 1;
    let kernel_stack_base = allocator
        .alloc_pages(page_count)
        .expect("Allocation failed!") as *mut u8;
    let kernel_stack_size = alloc::PAGE_SIZE * page_count;

    // ページテーブルの作成
    let page_table_ptr = allocator.alloc_pages(1).unwrap() as *mut usize;
    let page_table: &mut [usize] = unsafe { core::slice::from_raw_parts_mut(page_table_ptr, 512) };
    let flags = PageFlags::R as usize | PageFlags::W as usize | PageFlags::X as usize;

    // カーネル空間をマッピング
    let start_paddr = unsafe { &__kernel_base as *const u8 as usize };
    let end_paddr = unsafe { &alloc::__free_ram_end as *const u8 as usize };
    let mut paddr = start_paddr;
    while paddr < end_paddr {
        vmem::map_page(page_table, paddr, paddr, flags, allocator);
        paddr += alloc::PAGE_SIZE;
    }

    // ユーザーのマッピング
    // TODO: Allocatorが連続領域ではないところを返した場合に対応できない
    let user_flags = PageFlags::U as usize
        | PageFlags::R as usize
        | PageFlags::W as usize
        | PageFlags::X as usize;
    for maybe_seg in loaded.loadable_segments.iter() {
        if let Some(seg) = maybe_seg {
            // 必要なページ数を計算
            let pages_num = seg.memsz.div_ceil(alloc::PAGE_SIZE);
            // マッピング先の領域を取得
            let mut page_ptr = allocator.alloc_pages(pages_num).unwrap() as *mut u8;
            let page: &mut [u8] = unsafe { slice::from_raw_parts_mut(page_ptr, seg.filesz) };
            // ユーザープログラムのデータをコピー
            // 同じ長さでないとpanicする
            println!("copying user program dst={:p}", page);
            page[0..seg.filesz].copy_from_slice(seg.data);

            // TODO: 確保した領域が連続であることを前提にした実装
            let mut vaddr = seg.vaddr;
            if !is_aligned(vaddr, PAGE_SIZE) {
                vaddr = (vaddr & !0xFFF) + PAGE_SIZE;
            }
            for _ in 0..pages_num {
                vmem::map_page(page_table, vaddr, page_ptr as usize, user_flags, allocator);
                unsafe {
                    page_ptr = page_ptr.add(alloc::PAGE_SIZE);
                }
                vaddr += alloc::PAGE_SIZE;
            }
        }
    }

    unsafe {
        // ページングの有効化
        // satpレジスタの値はPTEと同様にページ番号で指定するのでPAGE_SIZEで割る
        let pt_number =
            vmem::SATP_SV39 | (page_table_ptr as *const usize as usize) / alloc::PAGE_SIZE;
        csr::write_csr(csr::Csr::Satp, pt_number);

        // 割り込み時のカーネルスタックのspの保存
        // 注意: u8 (1byte) の単位で加算する必要がある
        let kernel_stack_top = (kernel_stack_base as *mut u8).add(kernel_stack_size) as *mut usize;
        csr::write_csr(csr::Csr::Sscratch, kernel_stack_top as usize);

        // user_entryでsretしたときに最初に飛ぶアドレス
        csr::write_csr(csr::Csr::Sepc, loaded.entry_point);
    }

    // TODO: インデックスがPidになるのは一時的な実装
    proc.pid = Pid(idx);
    proc.state = ProcState::Runnable;
    proc.kernel_stack.base = kernel_stack_base;
    proc.kernel_stack.size = kernel_stack_size;
    proc.context.ra = user_entry as usize;
    proc.context.sp = proc.kernel_stack.top() as usize;
    proc.page_table = page_table;
}

//
// コンテキストスイッチとユーザーモード切替
//

const SSTATUS_SPIE: usize = 1 << 5;
extern "C" fn user_entry() {
    unsafe {
        asm!(
            "csrw sstatus, {0}",
            "sret",
            in(reg) SSTATUS_SPIE,
        );
    }
}

#[unsafe(naked)]
extern "C" fn switch_context(prev: *mut Context, next: *const Context) {
    naked_asm!(
        "sd ra, 0(a0)",
        "sd sp, 8(a0)",
        "sd s0, 16(a0)",
        "sd s1, 24(a0)",
        "sd s2, 32(a0)",
        "sd s3, 40(a0)",
        "sd s4, 48(a0)",
        "sd s5, 56(a0)",
        "sd s6, 64(a0)",
        "sd s7, 72(a0)",
        "sd s8, 80(a0)",
        "sd s9, 88(a0)",
        "sd s10, 96(a0)",
        "sd s11, 104(a0)",
        "ld ra, 0(a1)",
        "ld sp, 8(a1)",
        "ld s0, 16(a1)",
        "ld s1, 24(a1)",
        "ld s2, 32(a1)",
        "ld s3, 40(a1)",
        "ld s4, 48(a1)",
        "ld s5, 56(a1)",
        "ld s6, 64(a1)",
        "ld s7, 72(a1)",
        "ld s8, 80(a1)",
        "ld s9, 88(a1)",
        "ld s10, 96(a1)",
        "ld s11, 104(a1)",
        "ret",
    );
}

pub fn test_proc_switch() {
    println!("[test_proc_switch] calling yield_process()...");
    yield_process();
}
