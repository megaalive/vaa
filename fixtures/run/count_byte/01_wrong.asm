; intentionally wrong — empty body / missing loop (fails SemASM semantic or behavior)
BITS 64
DEFAULT REL
global count_byte
section .text
count_byte:
    ret
