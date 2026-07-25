bits 64
default rel
section .text
global add_base

add_base:
    push rbp
    mov rbp, rsp
    sub rsp, 16
    mov qword [rbp-8], rcx
    mov rax, qword [rbp-8]
    add rax, qword [rel base_value]
    mov rsp, rbp
    pop rbp
    ret

section .data
base_value dq 100
