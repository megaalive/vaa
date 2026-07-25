bits 64
default rel
section .text
global stack_local_i64

stack_local_i64:
    push rbp
    mov rbp, rsp
    sub rsp, 32
    mov qword [rbp-8], rcx
    mov r8, qword [rbp-8]
    mov qword [rbp-16], r8
    mov rax, qword [rbp-16]
    mov rsp, rbp
    pop rbp
    ret

section .data
