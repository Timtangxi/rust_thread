ENTRY(_start)

KERNEL_LOAD_ADDR = 0x40000000;

SECTIONS
{
  . = KERNEL_LOAD_ADDR;

  .text : ALIGN(4K) {
    __text_start = .;
    KEEP(*(.text.boot))
    KEEP(*(.vectors))
    *(.text .text.*)
    __text_end = .;
  }

  .rodata : ALIGN(4K) {
    __rodata_start = .;
    *(.rodata .rodata.*)
    __rodata_end = .;
  }

  .user_text : ALIGN(4K) {
    __user_text_start = .;
    KEEP(*(.user.text .user.text.*))
    __user_text_end = .;
  }

  .user_rodata : ALIGN(4K) {
    __user_rodata_start = .;
    KEEP(*(.user.rodata .user.rodata.*))
    __user_rodata_end = .;
  }

  .data : ALIGN(4K) {
    __data_start = .;
    *(.data .data.*)
    __data_end = .;
  }

  .bss (NOLOAD) : ALIGN(4K) {
    __bss_start = .;
    *(.bss .bss.*)
    *(COMMON)
    __bss_end = .;
  }

  . = ALIGN(16);
  __kernel_stack_bottom = .;
  . += 256K;
  __kernel_stack_top = .;

  . = ALIGN(16);
  __irq_stack_bottom = .;
  . += 64K;
  __irq_stack_top = .;

  . = ALIGN(16);
  __fiq_stack_bottom = .;
  . += 16K;
  __fiq_stack_top = .;

  . = ALIGN(16);
  __abort_stack_bottom = .;
  . += 64K;
  __abort_stack_top = .;

  . = ALIGN(16);
  __undefined_stack_bottom = .;
  . += 64K;
  __undefined_stack_top = .;

  . = ALIGN(4K);
  __kernel_end = .;
}
