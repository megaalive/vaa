; memcmp -- memory adversarial seed (Violated on ingest; mutator cannot repair).
BITS 64
DEFAULT REL
global memcmp
section .text
memcmp:
    mov byte [rcx], 0
    xor eax, eax
    ret
