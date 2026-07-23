; memcpy -- intentionally WRONG (Win64): returns 0 (status honored) but never
; copies src into dst. Mirrors `memcpy_wrong_win64.asm` in the SemASM
; `fixtures/asm/` corpus.
BITS 64
DEFAULT REL

global memcpy

section .text
memcpy:
    xor eax, eax
    ret
