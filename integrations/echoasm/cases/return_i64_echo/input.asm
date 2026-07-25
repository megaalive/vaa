; return_i64 — identity (Win64: rcx → rax).
BITS 64
DEFAULT REL

global return_i64

section .text
return_i64:
    mov rax, rcx
    ret
