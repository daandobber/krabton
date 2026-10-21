//! Compiles the vendored C++ of the Virus emulation (asmjit, the DSP56300
//! emulator, gearmulator's baseLib, synthLib and virusLib) and libresample,
//! with the small C interface Krabton calls. Always optimised: a DSP
//! emulator in a debug build cannot keep up with the sound.

use std::path::{Path, PathBuf};

fn sources(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            sources(&p, ext, out);
        } else if p.extension().is_some_and(|e| e == ext) {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            // Tests stay out (they have mains of their own, or need test data).
            let test = name.ends_with("_test.cpp") || name.contains("unittests") || name.contains("optimizertests") || name == "assemblertest.cpp";
            if !test {
                out.push(p);
            }
        }
    }
}

fn main() {
    let v = PathBuf::from("vendor");
    println!("cargo:rerun-if-changed=vendor");
    println!("cargo:rerun-if-changed=cpp");
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let msvc = env == "msvc";

    // libresample (C).
    let mut c = Vec::new();
    sources(&v.join("libresample/src"), "c", &mut c);
    cc::Build::new().files(&c).include(v.join("libresample/include")).include(v.join("libresample/src")).opt_level(3).debug(false).warnings(false).compile("krabton_resample");

    // Everything in C++, one library (no link order to get wrong).
    let mut cpp = Vec::new();
    sources(&v.join("asmjit/asmjit"), "cpp", &mut cpp);
    for lib in ["dsp56300/dsp56kBase", "dsp56300/dsp56kEmu", "gearmulator/baseLib", "gearmulator/synthLib", "gearmulator/virusLib"] {
        sources(&v.join(lib), "cpp", &mut cpp);
    }
    cpp.push(PathBuf::from("cpp/krabton_virus.cpp"));
    let mut b = cc::Build::new();
    b.cpp(true)
        .std("c++17")
        .opt_level(3)
        .debug(false)
        .warnings(false)
        .files(&cpp)
        .include(&v)
        .include(v.join("asmjit"))
        .include(v.join("dsp56300"))
        .include(v.join("gearmulator"))
        .define("ASMJIT_STATIC", None)
        .define("ASMJIT_NO_FOREIGN", None)
        .define("NDEBUG", None);
    if msvc {
        b.flag("/EHsc").flag("/permissive-").flag("/bigobj").flag("/utf-8").flag("/constexpr:steps10000000").flag("/fp:fast");
    } else {
        b.flag_if_supported("-fconstexpr-loop-limit=10000000").flag_if_supported("-fconstexpr-steps=10000000");
        b.flag_if_supported("-Wno-everything").flag("-fno-strict-aliasing");
    }
    b.compile("krabton_virus");

    match os.as_str() {
        "windows" => {
            for lib in ["shell32", "ole32", "user32", "advapi32", "synchronization", "avrt", "winmm"] {
                println!("cargo:rustc-link-lib={lib}");
            }
        }
        "macos" => {
            for fw in ["CoreFoundation", "AudioToolbox", "CoreAudio"] {
                println!("cargo:rustc-link-lib=framework={fw}");
            }
        }
        _ => {
            for lib in ["pthread", "dl", "rt"] {
                println!("cargo:rustc-link-lib={lib}");
            }
        }
    }
}
