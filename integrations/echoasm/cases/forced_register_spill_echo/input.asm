bits 64
default rel
section .text
global forced_register_spill

forced_register_spill:
    push rbp
    mov rbp, rsp
    sub rsp, 80
    mov qword [rbp-8], rcx
    mov r8, qword [rbp-8]
    mov qword [rbp-16], r8
    mov qword [rbp-24], 1
    mov qword [rbp-32], 2
    mov qword [rbp-40], 3
    mov qword [rbp-48], 4
    mov qword [rbp-56], 5
    mov qword [rbp-64], 6
    mov qword [rbp-72], 7
    mov r9, qword [rbp-24]
    add r9, qword [rbp-32]
    add r9, qword [rbp-40]
    add r9, qword [rbp-48]
    add r9, qword [rbp-56]
    add r9, qword [rbp-64]
    add r9, qword [rbp-72]
    mov rax, qword [rbp-16]
    mov rsp, rbp
    pop rbp
    ret

section .data
