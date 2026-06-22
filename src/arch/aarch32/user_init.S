.section .user.text, "ax"
.arm
.align 12
.global __user_init_entry

.equ SYS_SLEEP, 1
.equ SYS_EXIT, 2
.equ SYS_WRITE, 5
.equ SYS_READ, 6
.equ SYS_OPEN, 7
.equ SYS_CLOSE, 8
.equ O_CREAT, 0x40
.equ O_TRUNC, 0x200
.equ KERNEL_ABI_MAGIC, 0x80000000

__user_init_entry:
    ldr r7, =KERNEL_ABI_MAGIC

    mov r0, #SYS_WRITE
    mov r1, #1
    ldr r2, =__user_init_msg
    ldr r3, =__user_init_msg_len
    svc #0

    mov r0, #SYS_OPEN
    ldr r1, =__user_write_path
    ldr r2, =__user_write_path_len
    ldr r3, =O_CREAT + O_TRUNC
    svc #0
    cmp r0, #0
    blt 3f
    mov r4, r0

    mov r0, #SYS_WRITE
    mov r1, r4
    ldr r2, =__user_write_msg
    ldr r3, =__user_write_msg_len
    svc #0

    mov r0, #SYS_CLOSE
    mov r1, r4
    svc #0

    mov r0, #SYS_OPEN
    ldr r1, =__user_write_path
    ldr r2, =__user_write_path_len
    mov r3, #0
    svc #0
    cmp r0, #0
    blt 3f
    mov r4, r0

    mov r0, #SYS_READ
    mov r1, r4
    sub r6, sp, #96
    mov r2, r6
    mov r3, #32
    svc #0
    cmp r0, #0
    ble 2f
    mov r5, r0

    mov r0, #SYS_WRITE
    mov r1, #1
    ldr r2, =__user_readback
    ldr r3, =__user_readback_len
    svc #0

    mov r0, #SYS_WRITE
    mov r1, #1
    mov r2, r6
    mov r3, r5
    svc #0

2:
    mov r0, #SYS_CLOSE
    mov r1, r4
    svc #0

3:
    mov r0, #SYS_WRITE
    mov r1, #1
    ldr r2, =__user_prompt
    ldr r3, =__user_prompt_len
    svc #0

    mov r0, #SYS_READ
    mov r1, #0
    sub r6, sp, #64
    mov r2, r6
    mov r3, #64
    svc #0
    cmp r0, #0
    ble 4f

    mov r5, r0
    mov r0, #SYS_WRITE
    mov r1, #1
    ldr r2, =__user_echo
    ldr r3, =__user_echo_len
    svc #0

    mov r0, #SYS_WRITE
    mov r1, #1
    mov r2, r6
    mov r3, r5
    svc #0
    b 3b

4:
    mov r0, #SYS_SLEEP
    mov r1, #5
    svc #0
    b 3b

.ltorg

.section .user.rodata, "a"
.align 12
__user_init_msg:
    .ascii "[user-init] write(fd=1) from User mode via SVC\n"
.equ __user_init_msg_len, . - __user_init_msg

__user_write_path:
    .ascii "tmp/codex-write"
.equ __user_write_path_len, . - __user_write_path

__user_write_msg:
    .ascii "ramfs-write-ok\n"
.equ __user_write_msg_len, . - __user_write_msg

__user_readback:
    .ascii "[user-init] readback "
.equ __user_readback_len, . - __user_readback

__user_prompt:
    .ascii "user$ "
.equ __user_prompt_len, . - __user_prompt

__user_echo:
    .ascii "[user-shell] "
.equ __user_echo_len, . - __user_echo
