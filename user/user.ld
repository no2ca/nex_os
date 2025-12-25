ENTRY(start)

SECTIONS {
    . = 0x1000000;

    .text :{
        KEEP(*(.text.start));
        *(.text .text.*);
    }

    .rodata : ALIGN(4096) {
        *(.rodata .rodata.*);
    }

    .data : ALIGN(4096) {
        *(.data .data.*);
    }

    .bss : ALIGN(4096) {
        *(.bss .bss.* .sbss .sbss.*);

        . = ALIGN(16); /* https://github.com/nuta/operating-system-in-1000-lines/pull/23 */
        . += 64 * 1024; 
        __stack_top = .;
        . += 64 * 1024; 
       ASSERT(. < 0x1800000, "too large executable");
    }
}