; EchoAsm passthrough candidate — not Gate evidence.
; Demonstrates that generation is just bytes-in/bytes-out for this stub.
bits 64
global passthrough
passthrough:
    xor eax, eax
    ret
