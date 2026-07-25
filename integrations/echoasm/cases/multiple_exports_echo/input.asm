bits 64
default rel
section .text
global inc
global dec

inc:
    push rbp
    mov rbp, rsp
    sub rsp, 16
    mov qword [rbp-8], rcx
    mov rax, qword [rbp-8]
    add rax, 1
    mov rsp, rbp
    pop rbp
    ret

dec:
    push rbp
    mov rbp, rsp
    sub rsp, 16
    mov qword [rbp-8], rcx
    mov rax, qword [rbp-8]
    sub rax, 1
    mov rsp, rbp
    pop rbp
    ret

section .data
