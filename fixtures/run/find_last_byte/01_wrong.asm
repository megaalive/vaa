; intentionally wrong -- always returns 0 (fails absent -> length)
BITS 64
DEFAULT REL
global find_last_byte
section .text
find_last_byte:
    xor eax, eax
    ret
