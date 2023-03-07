    .section .text.entry
    .global _start
_start:
    la sp, tstack
    call rust_main
