; intentionally wrong — cmovl returns the SMALLER value.
; Microsoft x64: rcx=a, rdx=b, returns rax.
BITS 64
DEFAULT REL

global max_i64

section .text
max_i64:
    mov rax, rcx
    cmp rdx, rax
    cmovl rax, rdx
    ret
