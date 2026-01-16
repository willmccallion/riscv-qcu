/// Build script for qcu_hw crate.
///
/// Invokes Verilator to compile SystemVerilog RTL files into a C++ simulation
/// executable. Configures include paths, optimization level, and output
/// directory. Registers file dependencies to trigger rebuilds when RTL
/// sources change.
use std::env;
use std::path::PathBuf;
use std::process::Command;

/// Build script entry point for hardware simulation compilation.
///
/// Invokes Verilator to compile SystemVerilog RTL files into a C++ simulation
/// executable. Configures include paths for RTL modules, sets optimization
/// level to -O3, and registers file dependencies to trigger rebuilds when
/// RTL sources change. The resulting executable can be linked with Rust code
/// via FFI bindings.
fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let rtl_dir = manifest_dir.join("src/rtl");
    let sim_cpp = manifest_dir.join("src/sim/main.cpp");
    let top_sv = rtl_dir.join("top_soc.sv");

    let status = Command::new("verilator")
        .arg("--cc")
        .arg("--exe")
        .arg("--build")
        .arg("-O3")
        .arg("--Mdir")
        .arg(&out_dir)
        .arg("-Isrc/rtl")
        .arg("-Isrc/rtl/physics")
        .arg("-o")
        .arg("Vtop_soc_sim")
        .arg(&top_sv)
        .arg(&sim_cpp)
        .current_dir(&manifest_dir)
        .status()
        .expect("Failed to run verilator");

    if !status.success() {
        panic!("Verilator build failed.");
    }

    println!("cargo:rerun-if-changed=src/rtl/top_soc.sv");
    println!("cargo:rerun-if-changed=src/rtl/physics/hamiltonian_engine.sv");
    println!("cargo:rerun-if-changed=src/rtl/physics/qubit_grid.sv");
    println!("cargo:rerun-if-changed=src/sim/main.cpp");
}
