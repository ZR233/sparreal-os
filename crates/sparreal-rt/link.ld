OUTPUT_ARCH(%ARCH%)

INCLUDE "pie_boot.x"

ENTRY(_start)

SECTIONS
{
    _skernel = .;

    .text : ALIGN(4K) {
        _stext = .;
        *(.text.boot)
        *(.text .text.*)
        . = ALIGN(4K);
        _etext = .;
    } 

    .rodata : ALIGN(4K) {
        _srodata = .;
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
        *(.sdata2 .sdata2.*)
    } 
   
    .data : ALIGN(4K) {
        _erodata = .;
        _sdata = .;
        *(.data.boot .data.boot.*)
        . = ALIGN(4K);
        *(.data .data.*)
        *(.sdata .sdata.*)
        *(.got .got.*)
    } 

    DRIVERS : {  
        _sdriver = .;
        KEEP(*(.driver.register))
        _edriver = .;
    }

    .tdata : ALIGN(0x10) {
        _stdata = .;
        *(.tdata .tdata.*)
        _etdata = .;
    } 

    .tbss : ALIGN(0x10) {
        _stbss = .;
        *(.tbss .tbss.*)
        *(.tcommon)
        _etbss = .;
    } 

    . = ALIGN(4K);
    _edata = .;
    __kernel_load_end = .;

    .bss (NOLOAD) : ALIGN(4K) {
        _sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        *(COMMON)
        . = ALIGN(4K);
        _ebss = .;
    }

    _ekernel = .;
    __kernel_code_end = .;

    . = ALIGN(4K);
    . += 1M;
    _stack_bottom = .;
    _stack_top = . + %STACK_SIZE%;
	/DISCARD/ : {
        *(.comment) *(.gnu*) *(.note*) *(.eh_frame*)
    }
}


