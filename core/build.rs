fn main() {
    #[cfg(feature = "lapack")]
    build_fortran();
}

#[cfg(feature = "lapack")]
fn build_fortran() {
    use std::process::Command;

    let saved = save_and_clear_c_flags();

    cc::Build::new()
        .compiler("gfortran")
        .file("native/prism.f90")
        .file("native/spectral.f90")
        .flag("-fPIC")
        .flag("-O2")
        .compile("fortran_ops");

    restore_c_flags(saved);

    let output = Command::new("gfortran")
        .args(["-print-file-name=libgfortran.dylib"])
        .output()
        .expect("gfortran must be in PATH");
    let lib_path = String::from_utf8(output.stdout).unwrap();
    let lib_path = lib_path.trim();
    if let Some(dir) = std::path::Path::new(lib_path).parent() {
        println!("cargo:rustc-link-search=native={}", dir.display());
    }
    println!("cargo:rustc-link-lib=gfortran");

    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=framework=Accelerate");
    } else {
        println!("cargo:rustc-link-lib=lapack");
        println!("cargo:rustc-link-lib=blas");
    }
}

const C_FLAG_VARS: &[&str] = &["CFLAGS", "CXXFLAGS", "NIX_CFLAGS_COMPILE"];

fn save_and_clear_c_flags() -> Vec<(&'static str, Option<String>)> {
    C_FLAG_VARS
        .iter()
        .map(|&var| {
            let val = std::env::var(var).ok();
            unsafe { std::env::remove_var(var) };
            (var, val)
        })
        .collect()
}

fn restore_c_flags(saved: Vec<(&'static str, Option<String>)>) {
    for (var, val) in saved {
        match val {
            Some(v) => unsafe { std::env::set_var(var, v) },
            None => unsafe { std::env::remove_var(var) },
        }
    }
}
