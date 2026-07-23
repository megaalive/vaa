bits 64
default rel
section .text
global min_usize

min_usize:
    push rbp
    mov rbp, rsp
    sub rsp, 32
    mov qword [rbp-8], rcx
    mov qword [rbp-16], rdx
    mov r8, qword [rbp-8]
    mov r9, qword [rbp-16]
    cmp r8, r9
    jb then_0
    jmp else_0
then_0:
    mov rax, r8
    jmp endif_0
else_0:
    mov rax, r9
    jmp endif_0
endif_0:
    jmp cont_0
cont_0:
    mov rsp, rbp
    pop rbp
    ret

section .data
