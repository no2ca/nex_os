ENTRY(_start)

SECTIONS {
    . = 0x80200000;

    .text : ALIGN(4) {
        KEEP(*(.text._start));    /* .text._startを先頭にする */
        *(.text .text.*);
    }

    .rodata : ALIGN(4) {        /* 定数データ領域 */
        *(.rodata .rodata.*);
    }

    .data : ALIGN(4) {
        *(.data .data.*);
    }

    .bss : ALIGN(4) {
        __bss = .;
        *(.bss .bss.* .sbss .sbss.*);
        __bss_end = .;
    }

    . = ALIGN(4);
    . += 128 * 1024; /* 128KBのスタック領域 */
    __stack_top = .;
    
    . += 32 * 1024 * 1024; /* 32MB */
    . = ALIGN(4096);
    __free_ram = .;
    . += 32 * 1024 * 1024; /* 32MB */
    __free_ram_end = .;
    /* M-Modeで設定されるPMPにより物理メモリは0x8800_0000未満に制限される */
}
