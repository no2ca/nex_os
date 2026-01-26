ENTRY(_start)

/* TODO: 64bitなのでもしかしたら ALIGN(8) かもしれない */
SECTIONS {
    . = 0x80200000;
    __kernel_base = .;

    .text : ALIGN(8) {
        KEEP(*(.text._start));    /* .text._startを先頭にする */
        *(.text .text.*);
    }

    .rodata : ALIGN(8) {        /* 定数データ領域 */
        *(.rodata .rodata.*);
    }

    .data : ALIGN(8) {
        *(.data .data.*);
    }

    .bss : ALIGN(8) {
        __bss = .;
        *(.bss .bss.* .sbss .sbss.*);
        __bss_end = .;
    }

    . = ALIGN(8);
    . += 128 * 1024; /* 128KBのスタック領域 */
    __stack_top = .;
    . += 32 * 1024 * 1024; /* 32MB */

    . = ALIGN(4096); /* Allocatorが4096byte境界で常に返す */
    __page_area_start = .;
    . += 32 * 1024 * 1024; /* 32MB */
    __page_area_end = .;

    . = ALIGN(8);
    __heap_start = .;
    . += 32 * 1024 * 1024; /* 32MB */
    __heap_end = .;
    /* M-Modeで設定されるPMPにより物理メモリは0x8800_0000未満に制限される */
}
