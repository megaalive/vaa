; find_last_byte -- memory adversarial seed (Violated on ingest; mutator cannot repair).
BITS 64
DEFAULT REL
global find_last_byte
section .text
find_last_byte:
    mov byte [rcx], 0
    xor eax, eax
    ret
