; replace_byte -- intentionally WRONG (Win64): counts matches but never
; stores the replacement (no write side effect). Mirrors
; `replace_byte_wrong_win64.asm` in the SemASM `fixtures/asm/` corpus.
BITS 64
DEFAULT REL

global replace_byte

section .text
replace_byte:
    xor eax, eax
    test rdx, rdx
    jz .done
.loop:
    movzx r10d, byte [rcx]
    cmp r10b, r8b
    jne .skip
    inc rax
.skip:
    inc rcx
    dec rdx
    jnz .loop
.done:
    ret
