bits 64
default rel
section .text
global add_then_double

Double:
    push rbp
    mov rbp, rsp
    sub rsp, 16
    mov qword [rbp-8], rcx
    mov rax, qword [rbp-8]
    add rax, rax
    mov rsp, rbp
    pop rbp
    ret

AddThenStore:
    push rbp
    mov rbp, rsp
    sub rsp, 32
    mov qword [rbp-8], rcx
    mov qword [rbp-16], rdx
    mov rax, qword [rbp-8]
    add rax, qword [rbp-16]
    sub rsp, 32      ; shadow + stack args (16-byte aligned)
    mov rcx, rax
    call Double
    add rsp, 32      ; restore shadow + stack
    mov rsp, rbp
    pop rbp
    ret

add_then_double:
    push rbp
    mov rbp, rsp
    sub rsp, 32
    mov qword [rbp-8], rcx
    mov qword [rbp-16], rdx
    sub rsp, 32      ; shadow + stack args (16-byte aligned)
    mov rcx, [rbp-8]
    mov rdx, [rbp-16]
    call AddThenStore
    add rsp, 32      ; restore shadow + stack
    mov rsp, rbp
    pop rbp
    ret

section .data
