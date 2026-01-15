.section .text.init
.global _start
.global trap_vector

_start:
    # Disable interrupts globally to prevent early firing
    csrw mie, zero

    # Set the Machine Trap-Vector Base-Address Register (mtvec)
    la t0, trap_vector
    csrw mtvec, t0

    # Load Global Pointer (standard RISC-V relaxation)
    .option push
    .option norelax
    la gp, __global_pointer$
    .option pop

    # Get Hardware Thread ID (Hart ID)
    csrr t0, mhartid

    # Setup Stack Pointer
    li t1, 0x88000000

    # Stack size: 64KB (0x10000) per core
    li t2, 0x10000

    # Calculate offset: hartid * 64KB
    mul t2, t2, t0

    # Set SP: Top_of_RAM - Offset
    sub sp, t1, t2

    # Jump to Rust Kernel Main
    call kmain

    # Safety Loop (Should not be reached)
loop:
    wfi
    j loop

.align 4
trap_vector:
    # Allocate stack space for context saving
    # We need to save Caller-Saved registers (ra, t0-t6, a0-a7).
    # 16 registers * 8 bytes = 128 bytes.
    addi sp, sp, -128

    # Save Context
    sd ra,   0(sp)
    sd t0,   8(sp)
    sd t1,  16(sp)
    sd t2,  24(sp)
    sd a0,  32(sp)
    sd a1,  40(sp)
    sd a2,  48(sp)
    sd a3,  56(sp)
    sd a4,  64(sp)
    sd a5,  72(sp)
    sd a6,  80(sp)
    sd a7,  88(sp)
    sd t3,  96(sp)
    sd t4, 104(sp)
    sd t5, 112(sp)
    sd t6, 120(sp)

    # Call Rust Handler
    # The handler is defined in trap.rs as extern "C"
    call rust_trap_handler

    # Restore Context
    ld ra,   0(sp)
    ld t0,   8(sp)
    ld t1,  16(sp)
    ld t2,  24(sp)
    ld a0,  32(sp)
    ld a1,  40(sp)
    ld a2,  48(sp)
    ld a3,  56(sp)
    ld a4,  64(sp)
    ld a5,  72(sp)
    ld a6,  80(sp)
    ld a7,  88(sp)
    ld t3,  96(sp)
    ld t4, 104(sp)
    ld t5, 112(sp)
    ld t6, 120(sp)

    # Deallocate stack
    addi sp, sp, 128

    # Return from Machine Interrupt
    mret
