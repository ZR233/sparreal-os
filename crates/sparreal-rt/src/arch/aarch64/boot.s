.section ".text.head","ax"
.global _start

// Linux header
_start:
    // code0/code1
    nop
    bl _start_boot

    // text_offset
    .quad 0x80000

    // image_size
    .quad _kernel_size

    // flags
    .quad 0

    // Reserved fields
    .quad 0
    .quad 0
    .quad 0

    // magic - yes 0x644d5241 is the same as ASCII string "ARM\x64"
    .ascii "ARM\x64"

    // Another reserved field at the end of the header
    .byte 0, 0, 0, 0

.section .text.boot.start,"ax"
.align 3

// ------------------------------------------------------------

_start_boot:
  ADR      x11, .
  LDR      x10, =_start_boot
  SUB      x18, x10, x11 // x18 = va_offset

  MOV      x19, x0        // x19 = dtb_addr

  // // 获取当前CPU ID
  // MRS      x2, mpidr_el1
  // AND      x2, x2, #0xFF // 只保留CPU ID部分

  // // 设备树中的start cpuid存储在设备树的偏移量处
  // ADD      x3, x19, #0x28
  // LDR      x4, [x3] // x4 = start cpuid from dtb

  // // 比较当前CPU ID和start cpuid
  // CMP      x2, x4
  // BNE      .wfi_loop // 如果不相等，跳转到wfi循环

  LDR      x1, =_stack_top
  SUB      x1, x1, x18 // X1 == STACK_TOP
  MOV      sp, x1
  BL       __switch_to_el1


_el1_entry:
  // Install vector table
  // --------------------- 
  .global  vector_table_el1
  LDR      x0, =vector_table_el1
  MSR      VBAR_EL1, x0

  MOV      X1, #(0x3 << 20) // FPEN disables trapping to EL1.
  MSR      CPACR_EL1, X1
  ISB

  MOV      x0, x18
  MOV      x1, x19

  BL       __rust_boot

.wfi_loop:
  WFI // 等待中断
  B       .wfi_loop // 无限循环等待中断  