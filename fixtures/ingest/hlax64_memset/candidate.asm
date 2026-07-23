bits 64
default rel
section .text
global memset

memset:
    push rbp
    mov rbp, rsp
    sub rsp, 32
    mov qword [rbp-8], rcx
    mov qword [rbp-16], rdx
    mov qword [rbp-24], r8
    mov r9, 0
    mov r10, qword [rbp-8]
    jmp while_header_0
while_header_0:
    cmp r9, qword [rbp-16]
    jge endwhile_0
    jmp while_body_0
while_body_0:
    mov rax, [rbp-24]
    mov byte [r10], al
    add r9, 1
    add r10, 1
    jmp while_header_0
endwhile_0:
    jmp cont_0
cont_0:
    mov rax, 0
    mov rsp, rbp
    pop rbp
    ret

section .data
