OUTPUT_ARCH(riscv)
ENTRY(_start)
BASE_ADDRESS = 0x80200000;
HEAP_SIZE = 0x2000000;

SECTIONS
{
    .       = BASE_ADDRESS;
    skernel = .;
    stext   = .;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }

    .       = ALIGN(4K);
    etext   = .;
    srodata = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    .       = ALIGN(4K);
    erodata = .;
    sdata   = .;
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    .     = ALIGN(4K);
    edata = .;
    .bss : {
        sbss  = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        . = ALIGN(4K);
        ebss = .;
        ekernel = .;
        
        sheap = .;
        . += HEAP_SIZE;
        . = ALIGN(4K);
        eheap = .;

        bstack = .;
        . += 4K;
        tstack = .;
    }

    /DISCARD/ : {
        *(.eh_frame)
    }
}