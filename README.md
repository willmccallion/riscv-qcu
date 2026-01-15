# RISC-V Quantum Control Unit

This project implements a bare-metal firmware stack and hardware architecture designed for real-time Quantum Error Correction (QEC). It targets RISC-V soft-cores and focuses on deterministic execution, low-latency decoding, and hardware-software co-design.

The system decodes Surface Code errors using a custom instruction set, a zero-allocation Union-Find decoder, and a SystemVerilog hardware accelerator model.

## System Architecture

The architecture is divided into three logical layers:

### Core Logic
The core library contains the primary control logic and defines a custom bytecode (ISA) for quantum operations, including gate application, measurement, and decoding. The decoder implements the Union-Find algorithm with path compression and path halving optimizations. To ensure deterministic timing, the system uses a custom Bump Allocator backed by static memory regions, ensuring constant-time allocation and eliminating heap fragmentation.

### Firmware
The runtime environment is a `no_std` kernel designed for RV64IMAC architectures. It boots on bare metal (simulated via QEMU or FPGA) and manages system resources. Concurrency is handled through lock-free Single-Producer Single-Consumer (SPSC) ring buffers, which manage data flow between the I/O handling thread and the decoding thread without locking overhead.

### Hardware Acceleration
Specific decoder subroutines are offloaded to a SystemVerilog hardware model. The accelerator optimizes the `Find` operation of the Union-Find algorithm. The project includes a Verilator-based simulation harness, allowing Rust unit tests to drive the Verilog logic cycle-by-cycle for verification of the hardware design against the software reference.

## Technical Implementation

*   **Deterministic Memory Management:** The system avoids dynamic memory allocation during the decoding loop. All graph nodes, scratch buffers, and state vectors are pre-allocated in a contiguous memory arena to ensure cache locality and prevent latency spikes.
*   **Bit-Packed State Tracking:** Syndrome data and Pauli frames are managed using bit-packed state machines to reduce memory bandwidth requirements.
*   **Hardware-Software Co-Simulation:** The build system integrates Verilator, compiling SystemVerilog modules into C++ models. These are linked into the Rust test suite via FFI, enabling cycle-accurate verification within the software workflow.

## Performance

Benchmarks were conducted in two environments:
1.  **Host Simulation:** x86_64 workstation (Algorithm verification).
2.  **Target Simulation:** QEMU `virt` machine (RV64IMAC, 4 Cores).

### RISC-V Firmware (QEMU)
*   **Peak Throughput:** ~55,000 shots/s (3 Worker Cores, Saturated)
*   **Deterministic Latency:** ~580 Clock Cycles (Zero-Load)
    *   *@ 100 MHz (FPGA):* ~5.8 µs
    *   *@ 1 GHz (ASIC):* ~0.58 µs
*   **Jitter:** < 50 Cycles

### Host Simulation (x86)
*   **Single-Thread Throughput:** ~40,000 shots/s
*   **Service Time:** ~23 µs (d=5, p=0.005)

## Usage

A Python workflow script is provided to manage data generation, compilation, and simulation.

**Data Generation**
Generate a Surface Code circuit (Distance=5) with 10,000 shots using Stim. This embeds the synthetic syndrome data directly into the firmware source.
```bash
./scripts/run.py gen --size 5 --shots 10000
```

**Host Simulation**
Run the decoder logic on the host machine to establish baseline throughput and latency metrics.
```bash
./scripts/run.py stream --freq 100000
```

**Firmware Boot**
Compile the firmware for the `riscv64gc-unknown-none-elf` target and boot the kernel in QEMU.
```bash
./scripts/run.py kernel
```

**Hardware Verification**
Compile the SystemVerilog accelerator into a C++ model and run the Rust integration tests.
```bash
cargo test -p qcu_hw
```

## Dependencies

*   Rust Nightly Toolchain
*   QEMU (`qemu-system-riscv64`)
*   Verilator
*   Python 3 (with `stim` library)
*   RISC-V GCC Toolchain
