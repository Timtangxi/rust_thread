.section .text.context_switch, "ax"
.arm
.global context_restore

context_restore:
    mov sp, r0
    ldr r0, [sp]
    ldr r1, [sp, #4]
    cps #0x1f
    mov sp, r0
    mov lr, r1
    cps #0x13
    add sp, sp, #8
    pop {{r0-r12, lr}}
    rfefd sp!
