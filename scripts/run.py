#!/usr/bin/env python3
import argparse
import subprocess
import sys
import os

FIRMWARE_CRATE = "qcu_firmware"
HOST_CRATE = "qcu_host"
TARGET_ARCH = "riscv64gc-unknown-none-elf"

OUTPUT_DIR = "output"
DEM_FILE = os.path.join(OUTPUT_DIR, "bench.dem")
B8_FILE = os.path.join(OUTPUT_DIR, "bench.b8")
KERNEL_BIN = f"target/{TARGET_ARCH}/release/{FIRMWARE_CRATE}"

def run_cmd(cmd):
    print(f"[$] {cmd}")
    ret = subprocess.call(cmd, shell=True)
    if ret != 0:
        print(f"[!] Command failed: {cmd}")
        sys.exit(ret)

def ensure_data(size=5, shots=10000):
    if not os.path.exists(OUTPUT_DIR):
        os.makedirs(OUTPUT_DIR)

    # Generate Stim Data (Using Python Script now)
    if not os.path.exists(DEM_FILE) or not os.path.exists(B8_FILE):
        print("--> Generating benchmark data (Stim)...")
        # Call the python generator instead of cargo run
        run_cmd(f"python3 scripts/generate_stim_data.py --distance {size} --shots {shots} --out_dem {DEM_FILE} --out_b8 {B8_FILE}")

def build_firmware():
    print(f"--> Building {FIRMWARE_CRATE} (RISC-V)...")

    # Touch main.rs to force rebuild
    main_rs = f"crates/{FIRMWARE_CRATE}/src/main.rs"
    if os.path.exists(main_rs):
        os.utime(main_rs, None)

    run_cmd(f"cargo build --release -p {FIRMWARE_CRATE} --target {TARGET_ARCH} -Z build-std=core,alloc")

def run_qemu():
    print("--> Booting QEMU (SMP: 4 Cores)...")
    if not os.path.exists(KERNEL_BIN):
        print(f"[!] Kernel binary not found.")
        sys.exit(1)

    qemu_cmd = (
        f"qemu-system-riscv64 "
        f"-machine virt -m 128M -cpu rv64 -bios none -smp 4 "
        f"-nographic -serial mon:stdio "
        f"-kernel {KERNEL_BIN}"
    )
    run_cmd(qemu_cmd)

def run_stream_bench(freq):
    print("--> Running Host Stream Benchmark...")
    run_cmd(f"cargo run --release -p {HOST_CRATE} -- stream --dem {DEM_FILE} --b8 {B8_FILE} --freq {freq}")

def main():
    parser = argparse.ArgumentParser(description="QCU Workflow Manager")
    subparsers = parser.add_subparsers(dest="command", required=True)

    p_gen = subparsers.add_parser("gen", help="Generate benchmark data")
    p_gen.add_argument("--size", type=int, default=5)
    p_gen.add_argument("--shots", type=int, default=10000)

    p_kernel = subparsers.add_parser("kernel", help="Build and boot RISC-V firmware")
    p_kernel.add_argument("--size", type=int, default=5)

    p_stream = subparsers.add_parser("stream", help="Run host stream benchmark")
    p_stream.add_argument("--freq", type=int, default=80000)

    args = parser.parse_args()

    if args.command == "gen":
        ensure_data(args.size, args.shots)
    elif args.command == "kernel":
        ensure_data(args.size)
        build_firmware()
        run_qemu()
    elif args.command == "stream":
        ensure_data()
        run_stream_bench(args.freq)

if __name__ == "__main__":
    main()
