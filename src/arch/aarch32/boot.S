.section .text.boot, "ax"
.arm
.global _start
.global vectors

.equ MODE_SVC, 0x13
.equ MODE_IRQ, 0x12
.equ I_BIT, 0x80
.equ F_BIT, 0x40

_start:
    cpsid if

    cps #MODE_IRQ
    ldr sp, =__irq_stack_top

    cps #MODE_SVC
    ldr sp, =__kernel_stack_top

    ldr r0, =__bss_start
    ldr r1, =__bss_end
    mov r2, #0
1:
    cmp r0, r1
    bhs 2f
    str r2, [r0], #4
    b 1b

2:
    bl kernel_main
    b .

.section .vectors, "ax"
.align 5
vectors:
    ldr pc, =reset_vector
    ldr pc, =undefined_vector
    ldr pc, =svc_vector
    ldr pc, =prefetch_abort_vector
    ldr pc, =data_abort_vector
    ldr pc, =reserved_vector
    ldr pc, =irq_vector
    ldr pc, =fiq_vector

reset_vector:
    b _start

undefined_vector:
    sub lr, lr, #4
    mov r0, #1
    b exception_common

svc_vector:
    srsdb sp!, #MODE_SVC
    cps #MODE_SVC
    cpsid i
    push {{r0-r12, lr}}
    mov r0, sp
    bl svc_rust_entry
    mov sp, r0
    pop {{r0-r12, lr}}
    rfefd sp!

prefetch_abort_vector:
    sub lr, lr, #4
    mov r0, #3
    b exception_common

data_abort_vector:
    sub lr, lr, #8
    mov r0, #4
    b exception_common

reserved_vector:
    mov r0, #5
    b exception_common

fiq_vector:
    sub lr, lr, #4
    mov r0, #7
    b exception_common

exception_common:
    mov r1, lr
    mrs r2, spsr
    bl exception_rust_entry
    b .

irq_vector:
    sub lr, lr, #4
    srsdb sp!, #MODE_SVC
    cps #MODE_SVC
    push {{r0-r12, lr}}
    mov r0, sp
    bl irq_rust_entry
    mov sp, r0
    pop {{r0-r12, lr}}
    rfefd sp!
