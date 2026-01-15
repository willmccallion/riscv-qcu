// crates/qcu_hw/build.rs
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // 1. Run Verilator
    let status = Command::new("verilator")
        .arg("-cc")
        .arg("src/union_find.sv")
        .arg("--trace")
        .arg("--Mdir")
        .arg(&out_dir)
        .status()
        .expect("Failed to run verilator");

    if !status.success() {
        panic!("Verilator failed");
    }

    let verilator_root = match env::var("VERILATOR_ROOT") {
        Ok(v) => PathBuf::from(v),
        Err(_) => PathBuf::from("/usr/share/verilator"),
    };

    let mut build = cc::Build::new();

    build
        .cpp(true)
        .include(verilator_root.join("include"))
        .include(verilator_root.join("include/vltstd"))
        .include(&out_dir)
        .flag("-Wno-sign-compare")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-unused-variable") // Added
        .flag("-Wno-unused-but-set-variable"); // Added

    build.file(verilator_root.join("include/verilated.cpp"));
    build.file(verilator_root.join("include/verilated_vcd_c.cpp"));

    if verilator_root
        .join("include/verilated_threads.cpp")
        .exists()
    {
        build.file(verilator_root.join("include/verilated_threads.cpp"));
    }

    build.file("src/sim_main.cpp");

    let entries = fs::read_dir(&out_dir).expect("Failed to read OUT_DIR");
    for entry in entries {
        let entry = entry.expect("Error reading entry");
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "cpp" {
                let stem = path.file_stem().unwrap().to_str().unwrap();
                if stem.starts_with("Vunion_find") {
                    build.file(&path);
                }
            }
        }
    }

    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-link-lib=stdc++");

    build.compile("qcu_hw_sim");

    println!("cargo:rerun-if-changed=src/union_find.sv");
    println!("cargo:rerun-if-changed=src/sim_main.cpp");
}
