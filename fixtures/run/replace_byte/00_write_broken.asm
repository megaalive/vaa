; replace_byte -- Gate-1 adversarial seed (static Violated; mutator cannot repair).
; Indirect jump fails the leaf control-flow gate without --allow-execution.
; Write-shape leaves skip the static memory gate (SemASM ADR 0004), so an
; out-of-region store alone is Incomplete on Gate-1; guard-byte OOB evidence
; needs Gate-2 (--allow-execution). Incomplete ≠ Verified.
BITS 64
DEFAULT REL
global replace_byte
section .text
replace_byte:
    jmp rax
