bits 64
default rel
section .text
global point_sum

point_sum:
    push rbp
    mov rbp, rsp
    sub rsp, 48
    mov qword [rbp-8], rcx
    mov qword [rbp-16], rdx
    mov r8, qword [rbp-8]
    mov r9, qword [rbp-16]
    mov qword [rbp-32], r8
    mov qword [rbp-32+8], r9
    mov rax, qword [rbp-32]
    mov r10, qword [rbp-32+8]
    add rax, r10
    mov rsp, rbp
    pop rbp
    ret

section .data
