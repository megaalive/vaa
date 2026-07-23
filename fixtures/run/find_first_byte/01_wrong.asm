; intentionally wrong — always returns 0 (fails absent → length)
BITS 64
DEFAULT REL
global find_first_byte
section .text
find_first_byte:
    xor eax, eax
    ret
