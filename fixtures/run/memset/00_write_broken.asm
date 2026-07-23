; memset -- memory adversarial seed (Violated on ingest; mutator cannot repair).
; Writes one byte past the declared buffer[0..length) region (out-of-bounds
; write at buffer[length]), regardless of length -- violates the declared
; memory_write region "buffer[0..length]" even though `buffer` itself is a
; legitimate write target for this leaf.
BITS 64
DEFAULT REL
global memset
section .text
memset:
    mov byte [rcx + rdx], 0
    xor eax, eax
    ret
