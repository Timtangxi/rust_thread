ENTRY(_start)

KERNEL_LOAD_ADDR = 0x40000000;

SECTIONS
{
  . = KERNEL_LOAD_ADDR;

  .text : ALIGN(4K) {
    KEEP(*(.text.boot))
    KEEP(*(.vectors))
    *(.text .text.*)
  }

  .rodata : ALIGN(4K) {
    *(.rodata .rodata.*)
  }

  .data : ALIGN(4K) {
    *(.data .data.*)
  }

  .bss (NOLOAD) : ALIGN(4K) {
    __bss_start = .;
    *(.bss .bss.*)
    *(COMMON)
    __bss_end = .;
  }

  . = ALIGN(16);
  __kernel_stack_bottom = .;
  . += 64K;
  __kernel_stack_top = .;

  . = ALIGN(16);
  __irq_stack_bottom = .;
  . += 16K;
  __irq_stack_top = .;

  . = ALIGN(4K);
  __kernel_end = .;
}
