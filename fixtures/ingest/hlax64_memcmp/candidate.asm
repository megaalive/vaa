bits 64
default rel
section .text
global memcmp

memcmp:
    push rbp
    mov rbp, rsp
    sub rsp, 32
    mov qword [rbp-8], rcx
    mov qword [rbp-16], rdx
    mov qword [rbp-24], r8
    mov rax, 0
    mov r9, 0
    mov r10, qword [rbp-8]
    mov r14, qword [rbp-16]
    jmp while_header_0
while_header_0:
    cmp r9, qword [rbp-24]
    jge endwhile_0
    jmp while_body_0
while_body_0:
    movzx r11, byte [r10]
    movzx r12, byte [r14]
    cmp r11, r12
    jne then_1
    jmp else_1
then_1:
    cmp r11, r12
    jl then_2
    jmp else_2
else_1:
    add r9, 1
    add r10, 1
    add r14, 1
    jmp endif_1
then_2:
    mov rax, 0
    sub rax, 1
    jmp endif_2
else_2:
    mov rax, 1
    jmp endif_2
endif_2:
    jmp cont_2
cont_2:
    mov r9, qword [rbp-24]
    jmp endif_1
endif_1:
    jmp cont_1
cont_1:
    jmp while_header_0
endwhile_0:
    jmp cont_0
cont_0:
    mov rsp, rbp
    pop rbp
    ret

section .data
