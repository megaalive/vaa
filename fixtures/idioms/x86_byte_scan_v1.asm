; x86_byte_scan_v1 — CI-proven byte-scan skeleton (guidance copy)
BITS 64
DEFAULT REL
; xor eax,eax / test rdx,rdx / jz .done / .loop: cmp byte [rcx], r8b / …
