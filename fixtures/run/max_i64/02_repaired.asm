; max_i64 — larger (signed) of a and b via cmovg.
; Microsoft x64: rcx=a, rdx=b, returns rax.
; Requires SemASM tip that models cmov* as OpKind::Select (>= 0ab8004).
BITS 64
DEFAULT REL

global max_i64

section .text
max_i64:
    mov rax, rcx
    cmp rdx, rax
    cmovg rax, rdx
    ret
