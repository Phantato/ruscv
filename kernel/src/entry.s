    .section .text.entry
    .globl _start
_start:
    la sp, boot_stack_top
    call rust_main

    .section .boot_stack
    .globl boot_stack_lower_bound
    .globl boot_stack_top
boot_stack_lower_bound:
    .space 4096 * 1
boot_stack_top: