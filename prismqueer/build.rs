//! Build script: compiles Brainfuck programs to native Rust functions.
//!
//! Reads all `.bf` files from `brainfuck/`, applies IR optimizations
//! (run-length encoding, clear loops, copy loops), and generates
//! native Rust functions that produce identical output to the interpreter.

use std::env;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// IR — Intermediate representation for optimized BF
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum IR {
    /// Move data pointer right by n
    Right(usize),
    /// Move data pointer left by n
    Left(usize),
    /// Add n to current cell (wrapping)
    Add(u8),
    /// Subtract n from current cell (wrapping)
    Sub(u8),
    /// Read one byte of input
    Read,
    /// Write current cell to output
    Write,
    /// While loop: while tape[dp] != 0 { body }
    Loop(Vec<IR>),
    /// Clear current cell: tape[dp] = 0
    Clear,
    /// Copy current cell to dp+offset, then clear current: tape[dp+n] += tape[dp]; tape[dp] = 0
    CopyAdd(usize),
}

// ---------------------------------------------------------------------------
// Parsing: BF source → nested IR (unoptimized)
// ---------------------------------------------------------------------------

fn parse_bf(source: &str) -> Vec<u8> {
    source
        .bytes()
        .filter(|b| matches!(b, b'>' | b'<' | b'+' | b'-' | b'.' | b',' | b'[' | b']'))
        .collect()
}

fn build_ir(instructions: &[u8]) -> Vec<IR> {
    let mut stack: Vec<Vec<IR>> = vec![vec![]];

    for &inst in instructions {
        match inst {
            b'>' => stack.last_mut().unwrap().push(IR::Right(1)),
            b'<' => stack.last_mut().unwrap().push(IR::Left(1)),
            b'+' => stack.last_mut().unwrap().push(IR::Add(1)),
            b'-' => stack.last_mut().unwrap().push(IR::Sub(1)),
            b'.' => stack.last_mut().unwrap().push(IR::Write),
            b',' => stack.last_mut().unwrap().push(IR::Read),
            b'[' => stack.push(vec![]),
            b']' => {
                let body = stack.pop().expect("unmatched ]");
                stack.last_mut().unwrap().push(IR::Loop(body));
            }
            _ => {}
        }
    }

    assert!(stack.len() == 1, "unmatched [");
    stack.pop().unwrap()
}

// ---------------------------------------------------------------------------
// Optimization passes
// ---------------------------------------------------------------------------

fn optimize(ir: Vec<IR>) -> Vec<IR> {
    let ir = run_length_encode(ir);

    pattern_match(ir)
}

/// Run-length encode consecutive identical instructions.
fn run_length_encode(ir: Vec<IR>) -> Vec<IR> {
    let mut out = Vec::new();

    for op in ir {
        match op {
            IR::Right(n) => match out.last_mut() {
                Some(IR::Right(ref mut m)) => *m += n,
                _ => out.push(IR::Right(n)),
            },
            IR::Left(n) => match out.last_mut() {
                Some(IR::Left(ref mut m)) => *m += n,
                _ => out.push(IR::Left(n)),
            },
            IR::Add(n) => match out.last_mut() {
                Some(IR::Add(ref mut m)) => *m = m.wrapping_add(n),
                _ => out.push(IR::Add(n)),
            },
            IR::Sub(n) => match out.last_mut() {
                Some(IR::Sub(ref mut m)) => *m = m.wrapping_add(n),
                _ => out.push(IR::Sub(n)),
            },
            IR::Loop(body) => out.push(IR::Loop(run_length_encode(body))),
            other => out.push(other),
        }
    }

    out
}

/// Recognize clear loops [-] and copy loops [->+<].
fn pattern_match(ir: Vec<IR>) -> Vec<IR> {
    let mut out = Vec::new();

    for op in ir {
        match op {
            IR::Loop(ref body) => {
                // Clear loop: [-]
                if body.len() == 1 {
                    if let IR::Sub(1) = body[0] {
                        out.push(IR::Clear);
                        continue;
                    }
                }
                // Copy loop: [->...+<...]
                // Pattern: Sub(1), Right(n), Add(1), Left(n)
                if body.len() == 4 {
                    if let (IR::Sub(1), IR::Right(n), IR::Add(1), IR::Left(m)) =
                        (&body[0], &body[1], &body[2], &body[3])
                    {
                        if n == m {
                            out.push(IR::CopyAdd(*n));
                            continue;
                        }
                    }
                }
                // Recurse into non-matched loops
                out.push(IR::Loop(pattern_match(body.clone())));
            }
            other => out.push(other),
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Code generation: IR → Rust source
// ---------------------------------------------------------------------------

fn codegen(name: &str, ir: &[IR]) -> String {
    let fn_name = format!("{}_bf", name.replace('-', "_"));
    let mut code = String::new();

    code.push_str("#[allow(unused_assignments, unused_mut, unused_variables)]\n");
    code.push_str(&format!("pub fn {}(input: &[u8]) -> Vec<u8> {{\n", fn_name));
    code.push_str("    let mut tape = [0u8; 256];\n");
    code.push_str("    let mut dp: usize = 0;\n");
    code.push_str("    let mut inp: usize = 0;\n");
    code.push_str("    let mut output = Vec::new();\n");

    emit_ir(&mut code, ir, 1);

    code.push_str("    output\n");
    code.push_str("}\n");
    code
}

fn emit_ir(code: &mut String, ir: &[IR], indent: usize) {
    let pad = "    ".repeat(indent);
    for op in ir {
        match op {
            IR::Right(n) => {
                code.push_str(&format!("{}dp = (dp + {}).min(255);\n", pad, n));
            }
            IR::Left(n) => {
                code.push_str(&format!("{}dp = dp.saturating_sub({});\n", pad, n));
            }
            IR::Add(n) => {
                code.push_str(&format!(
                    "{}tape[dp] = tape[dp].wrapping_add({});\n",
                    pad, n
                ));
            }
            IR::Sub(n) => {
                code.push_str(&format!(
                    "{}tape[dp] = tape[dp].wrapping_sub({});\n",
                    pad, n
                ));
            }
            IR::Read => {
                code.push_str(&format!(
                    "{}tape[dp] = if inp < input.len() {{ input[inp] }} else {{ 0 }}; inp += 1;\n",
                    pad
                ));
            }
            IR::Write => {
                code.push_str(&format!("{}output.push(tape[dp]);\n", pad));
            }
            IR::Clear => {
                code.push_str(&format!("{}tape[dp] = 0;\n", pad));
            }
            IR::CopyAdd(offset) => {
                code.push_str(&format!(
                    "{}tape[dp + {}] = tape[dp + {}].wrapping_add(tape[dp]); tape[dp] = 0;\n",
                    pad, offset, offset
                ));
            }
            IR::Loop(body) => {
                code.push_str(&format!("{}while tape[dp] != 0 {{\n", pad));
                emit_ir(code, body, indent + 1);
                code.push_str(&format!("{}}}\n", pad));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Metal (MSL) code generation: IR → Metal Shading Language kernel
// ---------------------------------------------------------------------------

/// Emit a complete MSL kernel source from the given IR.
///
/// The generated kernel `<name>_bf` processes N inputs in parallel.
/// Each GPU thread handles one 22-byte input record
/// (16 features + 1 model index + 5 biases) and writes one output byte.
fn codegen_metal(name: &str, ir: &[IR]) -> String {
    let kernel_name = format!("{}_bf", name.replace('-', "_"));
    let mut msl = String::new();

    msl.push_str("#include <metal_stdlib>\n");
    msl.push_str("using namespace metal;\n\n");

    msl.push_str(&format!("kernel void {}(\n", kernel_name));
    msl.push_str("    device const uint8_t* input  [[buffer(0)]],\n");
    msl.push_str("    device       uint8_t* output [[buffer(1)]],\n");
    msl.push_str("    uint id [[thread_position_in_grid]]\n");
    msl.push_str(") {\n");
    msl.push_str("    uint base = id * 22;\n");
    msl.push_str("    uint8_t tape[256] = {0};\n");
    msl.push_str("    uint dp = 0;\n");
    msl.push_str("    uint inp = 0;\n");

    emit_ir_metal(&mut msl, ir, 1);

    msl.push_str("    output[id] = tape[28];\n");
    msl.push_str("}\n");
    msl
}

fn emit_ir_metal(msl: &mut String, ir: &[IR], indent: usize) {
    let pad = "    ".repeat(indent);
    for op in ir {
        match op {
            IR::Right(n) => {
                msl.push_str(&format!("{}dp = (dp + {}) % 256;\n", pad, n));
            }
            IR::Left(n) => {
                msl.push_str(&format!("{}dp = (dp + 256 - {}) % 256;\n", pad, n));
            }
            IR::Add(n) => {
                msl.push_str(&format!("{}tape[dp] = (tape[dp] + {}) & 0xFF;\n", pad, n));
            }
            IR::Sub(n) => {
                msl.push_str(&format!(
                    "{}tape[dp] = (tape[dp] + 256 - {}) & 0xFF;\n",
                    pad, n
                ));
            }
            IR::Read => {
                msl.push_str(&format!("{}tape[dp] = input[base + inp]; inp += 1;\n", pad));
            }
            IR::Write => {
                // Write is a no-op in the MSL kernel — result is read from tape[28] at the end.
            }
            IR::Clear => {
                msl.push_str(&format!("{}tape[dp] = 0;\n", pad));
            }
            IR::CopyAdd(offset) => {
                msl.push_str(&format!(
                    "{}tape[(dp + {}) % 256] = (tape[(dp + {}) % 256] + tape[dp]) & 0xFF; tape[dp] = 0;\n",
                    pad, offset, offset
                ));
            }
            IR::Loop(body) => {
                msl.push_str(&format!("{}while (tape[dp] != 0) {{\n", pad));
                emit_ir_metal(msl, body, indent + 1);
                msl.push_str(&format!("{}}}\n", pad));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Stats counting
// ---------------------------------------------------------------------------

fn count_ops(ir: &[IR]) -> usize {
    let mut n = 0;
    for op in ir {
        match op {
            IR::Loop(body) => n += 1 + count_ops(body),
            _ => n += 1,
        }
    }
    n
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    // Post-2026-07-18 fate pull-in: brainfuck/ lives at src/fate/brainfuck/
    // (Q4 scoped per Alex adjudication). build.rs stays at prismqueer root
    // (Q1 A monolithic per Alex "it's load bearing").
    let bf_dir = Path::new("src/fate/brainfuck");

    let mut generated = String::new();
    generated.push_str("// Generated by build.rs — BF-to-Rust compiler. Do not edit.\n\n");

    if bf_dir.exists() {
        let mut entries: Vec<_> = fs::read_dir(bf_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "bf"))
            .collect();
        entries.sort_by_key(|e| e.path());

        for entry in entries {
            let path = entry.path();
            let name = path.file_stem().unwrap().to_str().unwrap().to_string();
            let source = fs::read_to_string(&path).unwrap();
            let instructions = parse_bf(&source);
            let original_count = instructions.len();
            let ir = build_ir(&instructions);
            let optimized = optimize(ir);
            let optimized_count = count_ops(&optimized);
            let rust_code = codegen(&name, &optimized);
            generated.push_str(&rust_code);
            generated.push('\n');

            eprintln!(
                "  fate build.rs: {} — {} instructions -> {} optimized operations",
                path.display(),
                original_count,
                optimized_count,
            );

            // Tell cargo to rebuild if this file changes
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }

    let dest = Path::new(&out_dir).join("bf_compiled.rs");
    fs::write(&dest, generated).unwrap();

    // Also emit the Metal (MSL) kernel for the `metal` feature.
    // We generate it from the same IR as the Rust backend so they stay in sync.
    let mut metal_src = String::new();
    metal_src.push_str("// Generated by build.rs — BF-to-MSL compiler. Do not edit.\n\n");

    if bf_dir.exists() {
        let mut entries: Vec<_> = fs::read_dir(bf_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "bf"))
            .collect();
        entries.sort_by_key(|e| e.path());

        for entry in entries {
            let path = entry.path();
            let name = path.file_stem().unwrap().to_str().unwrap().to_string();
            let source = fs::read_to_string(&path).unwrap();
            let instructions = parse_bf(&source);
            let ir = build_ir(&instructions);
            let optimized = optimize(ir);
            let msl_code = codegen_metal(&name, &optimized);
            metal_src.push_str(&msl_code);
            metal_src.push('\n');
        }
    }

    let metal_dest = Path::new(&out_dir).join("fate.metal");
    fs::write(&metal_dest, metal_src).unwrap();

    // Also rerun if the brainfuck directory itself changes
    println!("cargo:rerun-if-changed=src/fate/brainfuck/");

    // ---------------------------------------------------------------
    // LAPACK feature branch (Reed 2026-07-20 per Alex FLANG-floor
    // directive: "This is literally the FLOOR, Reed. We don't
    // forward promise the FLOOR.").
    //
    // Under `--features lapack`: compile native/spectral.f90 + native/
    // prism.f90 with flang, archive into libspectral_native.a, and
    // emit link directives for LAPACK + BLAS + flang-rt runtime.
    //
    // Env vars supplied by mirror/flake.nix devShell (verified 2026-07-20):
    //   FLANG        = ${flang}/bin/flang
    //   FLANG_RT_DIR = ${flang-rt}/lib/clang/21/lib/darwin (darwin only)
    //   LAPACK_DIR   = ${pkgs.lapack}
    //   BLAS_DIR     = ${pkgs.blas}
    //   RUSTFLAGS    = -L $FLANG_RT_DIR -L $LAPACK_DIR/lib -L $BLAS_DIR/lib ...
    //
    // Fallback: `flang` / `ar` on PATH; LAPACK+BLAS via -L in RUSTFLAGS
    // (mirror-devshell already provides this).
    // ---------------------------------------------------------------
    if env::var_os("CARGO_FEATURE_LAPACK").is_some() {
        use std::process::Command;

        let flang = env::var("FLANG").unwrap_or_else(|_| "flang".to_string());
        let native_dir = Path::new("native");

        // Sanity check: native/ dir must exist with the two Fortran sources.
        assert!(
            native_dir.exists(),
            "prismqueer/native/ not found; expected spectral.f90 + prism.f90 \
             at {}. LAPACK feature requires the Fortran source tree.",
            native_dir.display()
        );

        let out = Path::new(&out_dir);
        let mut objs: Vec<std::path::PathBuf> = Vec::new();

        for src in &["spectral.f90", "prism.f90"] {
            let src_path = native_dir.join(src);
            assert!(
                src_path.exists(),
                "prismqueer/native/{} not found",
                src
            );
            let stem = src_path.file_stem().unwrap().to_str().unwrap();
            let obj_path = out.join(format!("{}.o", stem));

            let status = Command::new(&flang)
                .arg("-c")
                .arg("-fPIC")
                .arg("-O2")
                .arg(&src_path)
                .arg("-o")
                .arg(&obj_path)
                .status()
                .unwrap_or_else(|e| {
                    panic!(
                        "failed to invoke flang `{}` on {}: {}",
                        flang,
                        src_path.display(),
                        e
                    )
                });
            assert!(
                status.success(),
                "flang failed on {} (compiler: {})",
                src_path.display(),
                flang
            );

            objs.push(obj_path);
            println!("cargo:rerun-if-changed={}", src_path.display());
        }

        // Archive .o files into libspectral_native.a via `ar`.
        // Nix devshell provides AR env var; fallback to system `ar`.
        let ar = env::var("AR").unwrap_or_else(|_| "ar".to_string());
        let lib_path = out.join("libspectral_native.a");
        let _ = fs::remove_file(&lib_path);
        let mut ar_cmd = Command::new(&ar);
        ar_cmd.arg("rcs").arg(&lib_path);
        for o in &objs {
            ar_cmd.arg(o);
        }
        let status = ar_cmd
            .status()
            .unwrap_or_else(|e| panic!("failed to invoke ar `{}`: {}", ar, e));
        assert!(status.success(), "ar failed archiving {}", lib_path.display());

        // Link the Fortran static lib + LAPACK/BLAS/flang-rt.
        println!("cargo:rustc-link-search=native={}", out.display());
        println!("cargo:rustc-link-lib=static=spectral_native");

        // LAPACK + BLAS shared libs (nix-store paths via env vars).
        if let Ok(lapack_dir) = env::var("LAPACK_DIR") {
            println!("cargo:rustc-link-search=native={}/lib", lapack_dir);
        }
        if let Ok(blas_dir) = env::var("BLAS_DIR") {
            println!("cargo:rustc-link-search=native={}/lib", blas_dir);
        }
        println!("cargo:rustc-link-lib=lapack");
        println!("cargo:rustc-link-lib=blas");

        // flang-rt runtime static lib (darwin: libflang_rt.runtime.a).
        // Both -L (search path) and -l (link name) are needed for cargo to
        // resolve the archive; RUSTFLAGS from nix-devshell provides the
        // search path but we emit it here too for robustness across
        // invocation contexts.
        if let Ok(rt_dir) = env::var("FLANG_RT_DIR") {
            println!("cargo:rustc-link-search=native={}", rt_dir);
        }
        println!("cargo:rustc-link-lib=static=flang_rt.runtime");

        // Rebuild triggers.
        println!("cargo:rerun-if-env-changed=FLANG");
        println!("cargo:rerun-if-env-changed=FLANG_RT_DIR");
        println!("cargo:rerun-if-env-changed=LAPACK_DIR");
        println!("cargo:rerun-if-env-changed=BLAS_DIR");
        println!("cargo:rerun-if-env-changed=AR");
        println!("cargo:rerun-if-changed=native/");
    }
}
