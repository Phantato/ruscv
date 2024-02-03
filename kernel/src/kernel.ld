OUTPUT_ARCH(riscv)
ENTRY(_start)
BASE_ADDRESS = 0x80200000;

SECTIONS
{
    .       = BASE_ADDRESS;
    skernel = .;

    .text : {
        .           = ALIGN(4K);
        stext       = .;
        *(.text.entry)
        *(.text .text.*)
        .           = ALIGN(4K);
        strampoline = .;
        *(.trampoline)
        etext       = .;
    }

    .rodata : {
        .       = ALIGN(4K);
        srodata = .;
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
        erodata = .;
    }

    .data : {
        .       = ALIGN(4K);
        sdata   = .;
        *(.data .data.*)
        *(.sdata .sdata.*)
        edata   = .;
        .       = ALIGN(4K);

        bstack  = .;
        *(.boot_stack)
        .       = ALIGN(4K);
        tstack  = .;
    }

    .bss : {
        .       = ALIGN(4K);
        sbss    = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        ebss    = .;
    }

    .       = ALIGN(4K);
    ekernel = .;
    /DISCARD/ : {
        *(.eh_frame)
    }
}