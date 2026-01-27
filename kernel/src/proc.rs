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
    Exited,
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
    pt_number: usize,
    entry_point: usize,
}

impl Process {
    const fn unused() -> Self {
        Self {
            pid: Pid(usize::MAX),
            state: ProcState::Unused,
            kernel_stack: KernelStack::null(),
            context: Context::zero(),
            pt_number: 0,
            entry_point: 0,
        }
    }
}

//
// プロセステーブルの定義
//

use crate::allocator::PAGE_SIZE;
use crate::mem::{self, PageFlags};
use crate::utils::align_up;
use crate::{allocator, csr, loadelf, log_debug, log_info, log_trace, log_warn};
use core::arch::asm;
use core::{arch::naked_asm, cell::UnsafeCell};
use core::{slice, usize};

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

pub const NPROC: usize = 128;

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

    #[allow(unused)]
    #[inline]
    fn procs_ref(&self) -> &[Process; NPROC] {
        &self.procs
    }

    /// # Safety
    /// この呼び出し前に schedule() など, 内部のインデックスを変える操作を行っていないか
    #[allow(unused)]
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

pub fn dump_process_list(verdose: bool) {
    log_info!("proc", "process list:");
    let ptable = unsafe { PTABLE.get() };
    for proc in ptable.procs.iter() {
        if proc.state != ProcState::Unused || verdose {
            log_debug!(
                "proc",
                "pid={}, state={:?}, ra={:p}, entry={:p}, sp={:p}",
                proc.pid.as_usize(),
                proc.state,
                proc.context.ra as *const u8,
                proc.entry_point as *const u8,
                proc.context.sp as *const u8,
            );
        }
    }
}

//
// プロセス管理に関する内部の関数
//

static PTABLE: ProcessTableCell<ProcessTable> = ProcessTableCell::new(ProcessTable::new());

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
    log_warn!("scheduler", "No runnable process found");
    return &procs[0];
}

fn create_process_from_loaded(loaded: loadelf::LoadedElf) {
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
    let kernel_stack_base = allocator::PAGE_ALLOC
        .alloc_pages::<u8>(page_count)
        .as_mut_ptr();
    let kernel_stack_size = allocator::PAGE_SIZE * page_count;

    // ページテーブルの作成
    let page_table_ptr = allocator::PAGE_ALLOC.alloc_pages::<usize>(1).as_mut_ptr();
    let page_table = unsafe { core::slice::from_raw_parts_mut(page_table_ptr, 512) };

    // カーネル空間をマッピング
    map_kernel_pages(page_table);

    // ユーザー空間をマッピング
    map_user_pages(&loaded, page_table);

    let pt_number = mem::SATP_SV39 | (page_table_ptr as usize) / allocator::PAGE_SIZE;

    // TODO: インデックスがPidになるのは一時的な実装
    // → これはProcState::Exitedを導入して被らないようにしている
    // TODO: spにカーネルのスタックポインタを使用するとプロセス起動時に読めてしまう
    proc.pid = Pid(idx);
    proc.state = ProcState::Runnable;
    proc.kernel_stack.base = kernel_stack_base;
    proc.kernel_stack.size = kernel_stack_size;
    proc.context.ra = user_entry as usize;
    proc.context.sp = proc.kernel_stack.top() as usize;
    proc.pt_number = pt_number;
    proc.entry_point = loaded.entry_point;
}

/// カーネル空間のマッピングを行う関数
/// カーネルの最初からallocatorが確保できる領域の最後までを一対一でマップする
fn map_kernel_pages(page_table: &mut [usize]) {
    let flags = PageFlags::R | PageFlags::W | PageFlags::X;
    let start_paddr = unsafe { &__kernel_base as *const u8 as usize };
    let end_paddr = unsafe { &allocator::__heap_end as *const u8 as usize };
    let mut paddr = start_paddr;
    while paddr < end_paddr {
        mem::map_page(page_table, paddr, paddr, flags);
        paddr += allocator::PAGE_SIZE;
    }
}

/// ユーザーのマッピングを行う関数
/// allocatorが連続した領域を確保してくれることを前提にする
fn map_user_pages(loaded: &loadelf::LoadedElf, page_table: &mut [usize]) {
    for maybe_seg in loaded.loadable_segments.iter() {
        if let Some(seg) = maybe_seg {
            // 必要なページ数を計算
            let pages_num = seg.memsz.div_ceil(allocator::PAGE_SIZE);

            // マッピング先の領域を取得
            let page_ptr = allocator::PAGE_ALLOC
                .alloc_pages::<u8>(pages_num)
                .as_mut_ptr();
            let page: &mut [u8] = unsafe { slice::from_raw_parts_mut(page_ptr, seg.filesz) };

            // ユーザープログラムのデータをコピー
            log_debug!("proc", "copying user program dst={:p}", page);
            // セグメントの先頭がアラインされていない場合に同じ途中からの位置からコピーする
            let seg_start = seg.vaddr % PAGE_SIZE;
            // WARNING: 同じ長さでないとpanicする
            page[seg_start..seg_start + seg.filesz].copy_from_slice(seg.data);

            // ユーザーフラグの設定
            let user_flags = PageFlags::U | seg.flags;

            // ユーザ空間のマッピング
            let page_start_paddr = page_ptr as usize;
            let page_start_vaddr = align_up(seg.vaddr, PAGE_SIZE);
            log_debug!(
                "proc",
                "mapping vaddr={:#x} to paddr={:#x}, pages_num={}, flag={:?}",
                page_start_vaddr,
                page_start_paddr,
                pages_num,
                seg.flags
            );
            for i in 0..pages_num {
                let paddr = page_start_paddr + i * PAGE_SIZE;
                let vaddr = page_start_vaddr + i * PAGE_SIZE;
                mem::map_page(page_table, vaddr, paddr, user_flags);
            }
        }
    }
}

//
// コンテキストスイッチとユーザーモード切替
//

extern "C" fn user_entry() {
    unsafe {
        asm!(
            "csrw sstatus, {0}",
            "sret",
            in(reg) csr::SSTATUS_SPIE,
        );
    }
}

fn switch_context(prev: &mut Process, next: &Process) {
    let pt_number = next.pt_number;
    let kernel_stack_top = next.kernel_stack.top();
    let entry_point = next.entry_point;
    unsafe {
        // ページングの有効化
        csr::write_csr(csr::Csr::Satp, pt_number);
        // 割り込み時のカーネルスタックのspの保存
        csr::write_csr(csr::Csr::Sscratch, kernel_stack_top as usize);
        // user_entryでsretしたときに最初に飛ぶアドレス
        csr::write_csr(csr::Csr::Sepc, entry_point);
    }
    let prev_ctx = &mut prev.context;
    let next_ctx = &next.context;
    _swtch(prev_ctx, next_ctx);
}

#[unsafe(naked)]
extern "C" fn _swtch(prev: *mut Context, next: *const Context) {
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

#[unsafe(naked)]
extern "C" fn _start_proc(ctx: *const Context) {
    naked_asm!(
        "ld ra, 0(a0)",
        "ld sp, 8(a0)",
        "ld s0, 16(a0)",
        "ld s1, 24(a0)",
        "ld s2, 32(a0)",
        "ld s3, 40(a0)",
        "ld s4, 48(a0)",
        "ld s5, 56(a0)",
        "ld s6, 64(a0)",
        "ld s7, 72(a0)",
        "ld s8, 80(a0)",
        "ld s9, 88(a0)",
        "ld s10, 96(a0)",
        "ld s11, 104(a0)",
        "ret",
    );
}

//
// 外部に公開している関数
//

/// プロセスを生成する関数
pub fn create_process(elf_data: &'static [u8]) {
    let loaded = loadelf::load_elf(elf_data);
    create_process_from_loaded(loaded);
}

/// 現在のプロセス以外の実行可能プロセスに切り替える
///
/// 他に無い場合は同じプロセスが実行状態になる
pub fn yield_process() {
    let prev_proc = unsafe { PTABLE.get_mut().current_proc_mut_ref() };
    let next_proc = schedule();

    log_info!(
        "proc",
        "switching ... {:?} -> {:?}",
        prev_proc.pid,
        next_proc.pid
    );
    switch_context(prev_proc, next_proc);
}

/// 現在のプロセスを終了する関数
pub fn end_process() {
    // スケジュールより先に状態を変える必要がある
    let prev_proc = unsafe { PTABLE.get_mut().current_proc_mut_ref() };
    prev_proc.state = ProcState::Exited;

    let next_proc = schedule();

    log_info!(
        "proc",
        "switching ... {:?} (exited) -> {:?}",
        prev_proc.pid,
        next_proc.pid
    );

    switch_context(prev_proc, next_proc);
}

/// idleプロセスを作成する関数
pub fn create_idle_process() {
    let proc = unsafe { &mut PTABLE.get_mut().procs_mut()[0] };

    // カーネルスタック領域の取得
    let page_count = 1;
    let kernel_stack_base = allocator::PAGE_ALLOC
        .alloc_pages::<u8>(page_count)
        .as_mut_ptr();
    let kernel_stack_size = allocator::PAGE_SIZE * page_count;

    // ページテーブルの作成
    let page_table_ptr = allocator::PAGE_ALLOC.alloc_pages::<usize>(1).as_mut_ptr();
    let page_table: &mut [usize] = unsafe { core::slice::from_raw_parts_mut(page_table_ptr, 512) };
    let pt_number =
        mem::SATP_SV39 | (page_table_ptr as *const usize as usize) / allocator::PAGE_SIZE;

    // カーネル空間をマッピング
    map_kernel_pages(page_table);

    proc.pid = Pid(0);
    proc.state = ProcState::Runnable;
    proc.kernel_stack.base = kernel_stack_base;
    proc.kernel_stack.size = kernel_stack_size;
    proc.context.ra = idle_process as usize;
    proc.context.sp = proc.kernel_stack.top() as usize;
    proc.pt_number = pt_number;
}

/// idleプロセスで実行される関数
#[allow(unused)]
fn idle_process() {
    log_debug!("proc", "idling...");
    loop {
        core::hint::spin_loop();
    }
}

/// カーネルがプロセスを開始する際に使用する関数
///
/// コンテキストスイッチとの違いは前のプロセスが無いこと
pub fn start_process() {
    let next = schedule();

    if next.pid == Pid(0) {
        panic!("[start_process] no process to start")
    }

    let pt_number = next.pt_number;
    let kernel_stack_top = next.kernel_stack.top();
    let entry_point = next.entry_point;
    unsafe {
        // ページングの有効化
        csr::write_csr(csr::Csr::Satp, pt_number);
        // 割り込み時のカーネルスタックのspの保存
        csr::write_csr(csr::Csr::Sscratch, kernel_stack_top as usize);
        // user_entryでsretしたときに最初に飛ぶアドレス
        csr::write_csr(csr::Csr::Sepc, entry_point);
    }
    let ctx = &next.context;
    _start_proc(ctx);
}
