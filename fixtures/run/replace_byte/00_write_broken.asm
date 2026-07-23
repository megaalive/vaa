; replace_byte -- Gate-1 adversarial seed (static Violated; mutator cannot repair).
; Indirect jump fails the leaf control-flow gate without --allow-execution.
; Trailing `ret` satisfies nop-before-ret mutator parsing (unreachable).
; Write-shape skips the static memory gate (SemASM ADR 0004); OOB-store-only
; seeds are Incomplete on Gate-1. Incomplete ≠ Verified.
BITS 64
DEFAULT REL
global replace_byte
section .text
replace_byte:
    jmp rax
    ret
