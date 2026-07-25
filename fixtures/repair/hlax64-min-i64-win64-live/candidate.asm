bits 64
default rel
section .text
global min_i64

min_i64:
    push rbp
    mov rbp, rsp
    sub rsp, 32
    mov qword [rbp-8], rcx
    mov qword [rbp-16], rdx
    ; ir:1
    mov r8, qword [rbp-8]
    ; ir:2
    mov r9, qword [rbp-16]
    ; ir:3
    cmp r8, r9
    ; ir:4
    jg then_0
    ; ir:5
    jmp else_0
then_0:
    ; ir:6
    mov rax, r8
    ; ir:7
    jmp endif_0
else_0:
    ; ir:8
    mov rax, r9
    ; ir:9
    jmp endif_0
endif_0:
    ; ir:10
    jmp cont_0
cont_0:
    mov rsp, rbp
    pop rbp
    ret

section .data
