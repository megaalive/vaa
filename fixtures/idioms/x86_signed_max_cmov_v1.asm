; x86_signed_max_cmov_v1 — CI-proven max_i64 pattern (guidance copy)
BITS 64
DEFAULT REL
; mov rax, rcx / cmp rdx, rax / cmovg rax, rdx / ret
