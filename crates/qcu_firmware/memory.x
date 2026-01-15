MEMORY
{
  RAM : ORIGIN = 0x80000000, LENGTH = 128M
}

ENTRY(_start)

SECTIONS
{
  . = ORIGIN(RAM);

  .text : {
    KEEP(*(.text.init))
    *(.text .text.*)
  } > RAM

  .rodata : {
    *(.rodata .rodata.*)
  } > RAM

  .data : {
    . = ALIGN(8);
    __global_pointer$ = . + 0x800;
    *(.sdata .sdata.*)
    *(.data .data.*)
  } > RAM

  .bss : {
    . = ALIGN(8);
    _sbss = .;
    *(.sbss .sbss.*)
    *(.bss .bss.*)
    . = ALIGN(8);
    _ebss = .;
  } > RAM
}
