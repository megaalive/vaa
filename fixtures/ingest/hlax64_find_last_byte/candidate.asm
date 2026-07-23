bits 64
default rel
section .text
global find_last_byte

find_last_byte:
    push rbp
    mov rbp, rsp
    sub rsp, 32
    mov qword [rbp-8], rcx
    mov qword [rbp-16], rdx
    mov qword [rbp-24], r8
    mov rax, qword [rbp-16]
    mov r9, 0
    mov r10, qword [rbp-8]
    jmp while_header_0
while_header_0:
    cmp r9, qword [rbp-16]
    jge endwhile_0
    jmp while_body_0
while_body_0:
    movzx r11, byte [r10]
    cmp r11, qword [rbp-24]
    je then_1
    jmp cont_1
then_1:
    mov rax, r9
    jmp endif_1
endif_1:
    jmp cont_1
cont_1:
    add r9, 1
    add r10, 1
    jmp while_header_0
endwhile_0:
    jmp cont_0
cont_0:
    mov rsp, rbp
    pop rbp
    ret

section .data
