; intentionally wrong — subtracts instead of adds (behavioral violation).
; Microsoft x64: rcx=values, rdx=length, returns rax.
BITS 64
DEFAULT REL

global sum_i64

section .text
sum_i64:
    xor eax, eax        ; sum = 0
    test rdx, rdx
    jz .done
.loop:
    sub rax, [rcx]      ; BUG: should be add
    add rcx, 8
    dec rdx
    jnz .loop
.done:
    ret
