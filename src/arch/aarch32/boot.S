.section .text.boot, "ax"
.arm
.global _start
.global vectors

.equ MODE_SVC, 0x13
.equ MODE_IRQ, 0x12
.equ MODE_FIQ, 0x11
.equ MODE_ABT, 0x17
.equ MODE_UND, 0x1b
.equ MODE_SYS, 0x1f
.equ I_BIT, 0x80
.equ F_BIT, 0x40

_start:
    cpsid if
    ldr r3, =boot_fdt_ptr
    str r2, [r3]

    cps #MODE_IRQ
    ldr sp, =__irq_stack_top

    cps #MODE_FIQ
    ldr sp, =__fiq_stack_top

    cps #MODE_ABT
    ldr sp, =__abort_stack_top

    cps #MODE_UND
    ldr sp, =__undefined_stack_top

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
    sub sp, sp, #8
    cps #MODE_SYS
    mov r0, sp
    mov r1, lr
    cps #MODE_SVC
    str r0, [sp]
    str r1, [sp, #4]
    mov r0, sp
    bl svc_rust_entry
    mov sp, r0
    ldr r0, [sp]
    ldr r1, [sp, #4]
    cps #MODE_SYS
    mov sp, r0
    mov lr, r1
    cps #MODE_SVC
    add sp, sp, #8
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
    sub sp, sp, #8
    cps #MODE_SYS
    mov r0, sp
    mov r1, lr
    cps #MODE_SVC
    str r0, [sp]
    str r1, [sp, #4]
    mov r0, sp
    bl irq_rust_entry
    mov sp, r0
    ldr r0, [sp]
    ldr r1, [sp, #4]
    cps #MODE_SYS
    mov sp, r0
    mov lr, r1
    cps #MODE_SVC
    add sp, sp, #8
    pop {{r0-r12, lr}}
    rfefd sp!

.section .data.boot, "aw"
.global boot_fdt_ptr
.align 2
boot_fdt_ptr:
    .word 0
