; add_i64 — wrapping sum (Win64: rcx + rdx → rax).
BITS 64
DEFAULT REL

global add_i64

section .text
add_i64:
    mov rax, rcx
    add rax, rdx
    ret
