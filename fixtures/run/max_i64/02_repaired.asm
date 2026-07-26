; max_i64 — larger (signed) of a and b.
; Microsoft x64: rcx=a, rdx=b, returns rax.
; Branch instead of cmov: SemASM's semantic lowering models cmp/jcc/mov,
; while cmov currently lowers as unknown (require_complete_lowering fails).
BITS 64
DEFAULT REL

global max_i64

section .text
max_i64:
    mov rax, rcx
    cmp rdx, rax
    jle .done           ; keep a when b <= a
    mov rax, rdx        ; take b when b > a
.done:
    ret
