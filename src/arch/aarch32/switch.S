.section .text.context_switch, "ax"
.arm
.global context_restore

context_restore:
    mov sp, r0
    pop {{r0-r12, lr}}
    rfefd sp!
