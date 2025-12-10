ENTRY(_start)

SECTIONS {
    . = 0x80200000;

    .text :{
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
}
