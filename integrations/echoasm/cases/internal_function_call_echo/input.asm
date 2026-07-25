bits 64
default rel
section .text
global scale_by_two

Double:
    push rbp
    mov rbp, rsp
    sub rsp, 16
    mov qword [rbp-8], rcx
    mov rax, qword [rbp-8]
    add rax, rax
    mov rsp, rbp
    pop rbp
    ret

scale_by_two:
    push rbp
    mov rbp, rsp
    sub rsp, 16
    mov qword [rbp-8], rcx
    sub rsp, 32      ; shadow + stack args (16-byte aligned)
    mov rcx, [rbp-8]
    call Double
    add rsp, 32      ; restore shadow + stack
    mov rsp, rbp
    pop rbp
    ret

section .data
