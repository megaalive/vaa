; memset -- intentionally WRONG (Win64): returns 0 (status honored) but never
; stores to the buffer. Mirrors `memset_wrong_win64.asm` in the SemASM
; `fixtures/asm/` corpus.
BITS 64
DEFAULT REL

global memset

section .text
memset:
    xor eax, eax
    ret
