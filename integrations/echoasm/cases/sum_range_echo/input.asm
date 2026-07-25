; sum_range — wrapping triangular 0+1+…+n (Win64: rcx=n → rax).
BITS 64
DEFAULT REL

global sum_range

section .text
sum_range:
    xor eax, eax
    xor edx, edx
.L:
    cmp edx, ecx
    jg .done
    add eax, edx
    inc edx
    jmp .L
.done:
    ret
