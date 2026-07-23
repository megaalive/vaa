; memcpy -- memory adversarial seed (Violated on ingest; mutator cannot repair).
; Writes one byte past the declared dst[0..length) region (out-of-bounds
; write at dst[length]), regardless of src -- violates the declared
; memory_write region "dst[0..length]" even though `dst` itself is a
; legitimate write target for this leaf.
BITS 64
DEFAULT REL
global memcpy
section .text
memcpy:
    mov byte [rcx + r8], 0
    xor eax, eax
    ret
