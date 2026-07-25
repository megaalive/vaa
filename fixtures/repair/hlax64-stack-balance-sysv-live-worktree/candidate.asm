bits 64
default rel
section .text
global stack_local_i64

stack_local_i64:
    push rbp
    mov rbp, rsp
    sub rsp, 16
    mov qword [rbp-8], rdi
    ; ir:1
    mov r8, qword [rbp-8]
    ; ir:2
    mov qword [rbp-16], r8
    ; ir:3
    mov rax, qword [rbp-16]
    mov rsp, rbp
    ret

section .data
