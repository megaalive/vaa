bits 64
default rel
section .text
global replace_byte

replace_byte:
    push rbp
    mov rbp, rsp
    sub rsp, 48
    mov qword [rbp-8], rcx
    mov qword [rbp-16], rdx
    mov qword [rbp-24], r8
    mov qword [rbp-32], r9
    mov r8, 0
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
    mov rax, [rbp-32]
    mov byte [r10], al
    add r8, 1
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
    mov rax, r8
    mov rsp, rbp
    pop rbp
    ret

section .data
