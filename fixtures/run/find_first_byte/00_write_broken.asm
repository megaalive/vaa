; find_first_byte -- memory adversarial seed (Violated on ingest; mutator cannot repair).
BITS 64
DEFAULT REL
global find_first_byte
section .text
find_first_byte:
    mov byte [rcx], 0
    xor eax, eax
    ret
