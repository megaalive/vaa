; countdown_loop — count n down to 0; return original n (Win64: rcx).
BITS 64
DEFAULT REL

global countdown_loop

section .text
countdown_loop:
    mov rax, rcx
    mov rdx, rcx
    test rdx, rdx
    jle .done
.L:
    dec rdx
    jnz .L
.done:
    ret
