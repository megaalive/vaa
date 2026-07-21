bits 64
default rel
section .text
global sum_i64

sum_i64:
    push rbp
    mov rbp, rsp
    sub rsp, 32
    mov qword [rbp-8], rcx
    mov qword [rbp-16], rdx
    mov rax, 0
    mov r8, 0
    mov r10, qword [rbp-8]
    jmp while_header_0
while_header_0:
    cmp r8, qword [rbp-16]
    jge endwhile_0
    jmp while_body_0
while_body_0:
    mov r9, qword [r10]
    add rax, r9
    add r8, 1
    add r10, 8
    jmp while_header_0
endwhile_0:
    jmp cont_0
cont_0:
    mov rsp, rbp
    pop rbp
    ret

section .data
