use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("cyclonedds-sys must live under the workspace root");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    let prebuilt = manifest_dir.join("src/prebuilt_bindings.rs");

    // Try to find or build CycloneDDS C library.
    // If vendor/cyclonedds source is missing and no system library is found,
    // we still generate bindings from prebuilt file (for code-checking purposes).
    let mut probe_ctx: Option<(PathBuf, PathBuf)> = None;
    let lib_found = if let Ok(cyclonedds_src) = try_resolve_cyclonedds_source(workspace_root) {
        let cyclonedds_build = resolve_cyclonedds_build_dir(&cyclonedds_src, &out_dir);
        probe_ctx = Some((cyclonedds_src.clone(), cyclonedds_build.clone()));

        // Check if already built, otherwise try to build (requires cmake)
        if find_ddsc_library(&cyclonedds_build).is_some() {
            if let Some((lib_dir, link_kind)) = find_ddsc_library(&cyclonedds_build) {
                println!(
                    "cargo:warning=Using pre-built CycloneDDS from {}",
                    lib_dir.display()
                );
                emit_link_info(&lib_dir, link_kind);
                true
            } else {
                false
            }
        } else if env::var_os("CYCLONEDDS_BUILD").is_some() {
            // CYCLONEDDS_BUILD is set but library not found - this is an error
            println!(
                "cargo:warning=CYCLONEDDS_BUILD is set but library not found at {}",
                cyclonedds_build.display()
            );
            false
        } else if which_cmake().is_some() {
            ensure_cyclonedds_build_ready(&cyclonedds_src, &cyclonedds_build);
            if let Some((lib_dir, link_kind)) = find_ddsc_library(&cyclonedds_build) {
                println!(
                    "cargo:warning=Using freshly built CycloneDDS from {}",
                    lib_dir.display()
                );
                emit_link_info(&lib_dir, link_kind);
                true
            } else {
                false
            }
        } else {
            // cmake not available — fall back to system library search
            if let Some((lib_dir, link_kind)) = find_system_ddsc_library() {
                println!(
                    "cargo:warning=Using system CycloneDDS from {}",
                    lib_dir.display()
                );
                emit_link_info(&lib_dir, link_kind);
                true
            } else {
                println!("cargo:warning=cmake not found and no system CycloneDDS — generating bindings without linking");
                println!("cargo:rustc-link-lib=dylib=ddsc");
                false
            }
        }
    } else {
        // No source available — try to find system-installed library
        if let Some((lib_dir, link_kind)) = find_system_ddsc_library() {
            println!(
                "cargo:warning=Using system CycloneDDS from {}",
                lib_dir.display()
            );
            emit_link_info(&lib_dir, link_kind);
            true
        } else {
            println!(
                "cargo:warning=CycloneDDS library not found — generating bindings without linking"
            );
            // Emit a dummy link so downstream code compiles for checking
            println!("cargo:rustc-link-lib=dylib=ddsc");
            false
        }
    };

    let _ = lib_found; // suppress unused warning

    println!("cargo:rerun-if-env-changed=CYCLONEDDS_SRC");
    println!("cargo:rerun-if-env-changed=CYCLONEDDS_BUILD");
    println!("cargo:rerun-if-env-changed=CYCLONEDDS_STATIC");
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=build.rs");

    // Use prebuilt bindings (generated on macOS/Linux where bindgen works correctly
    // with the CycloneDDS headers). We skip bindgen entirely because clang on
    // Windows/MSVC fails to resolve many types that are correctly resolved on
    // other platforms.
    let bindings_path = out_dir.join("bindings.rs");

    if prebuilt.exists() {
        // FFI-ABI-008: os asserts de layout EMBUTIDOS no prebuilt foram gerados
        // para os tipos de libc do macOS/Linux (ex.: __darwin_pthread_*) e não
        // compilam em outros targets — por isso o strip abaixo permanece. A
        // verificação de ABI que importa (tipos da API pública do CycloneDDS
        // que o crate toca por valor) é feita pelo ABI probe em
        // `run_abi_probe`, executado por target, que FALHA o build em mismatch.
        let content = std::fs::read_to_string(&prebuilt).expect("couldn't read prebuilt bindings");
        let stripped = strip_static_assertions_from_str(&content);
        std::fs::write(&bindings_path, stripped).expect("couldn't write bindings");
    } else {
        panic!(
            "No prebuilt bindings found at {}. Run bindgen on macOS/Linux first.",
            prebuilt.display()
        );
    }

    // FFI-ABI-008: verifica size/align/offset dos tipos DDS críticos contra o
    // header/biblioteca nativos DESTE target. Falha o build em mismatch.
    run_abi_probe(probe_ctx.as_ref(), &out_dir);
}

fn emit_link_info(lib_dir: &Path, link_kind: &'static str) {
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib={link_kind}=ddsc");
    if link_kind == "dylib" {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
    }
    // Libs de sistema exigidas pelo CycloneDDS ao linkar ESTATICO (o .so ja as traz).
    if link_kind == "static" {
        #[cfg(target_os = "linux")]
        {
            println!("cargo:rustc-link-lib=pthread");
            println!("cargo:rustc-link-lib=dl");
            println!("cargo:rustc-link-lib=rt");
            println!("cargo:rustc-link-lib=m");
        }
        #[cfg(target_os = "macos")]
        println!("cargo:rustc-link-lib=pthread");
    }
    // Windows system libraries required by CycloneDDS
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-lib=bcrypt");
        println!("cargo:rustc-link-lib=iphlpapi");
        println!("cargo:rustc-link-lib=ws2_32");
    }
}

fn find_system_ddsc_library() -> Option<(PathBuf, &'static str)> {
    let search_paths = vec![
        PathBuf::from("/usr/lib"),
        PathBuf::from("/usr/local/lib"),
        PathBuf::from("/usr/lib/x86_64-linux-gnu"),
    ];
    for dir in search_paths {
        if let Some(result) = find_ddsc_library(&dir) {
            return Some(result);
        }
    }
    None
}

fn which_cmake() -> Option<PathBuf> {
    // Try `which cmake` first (Unix), fall back to `where cmake` (Windows)
    let output = Command::new("which")
        .arg("cmake")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .or_else(|| {
            Command::new("where")
                .arg("cmake")
                .output()
                .ok()
                .filter(|o| o.status.success())
        })?;
    let path = String::from_utf8_lossy(&output.stdout);
    Some(PathBuf::from(path.lines().next().unwrap_or("").trim()))
}

/// Remove `const _: () = { ... }` static assertion blocks from a string.
///
/// These blocks contain size/alignment/offset checks that fail at compile time when
/// the platform differs from where bindings were generated. They are not needed for
/// the crate to function correctly.
fn strip_static_assertions_from_str(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut depth: i32 = 0;
    let mut in_assertion = false;

    for line in content.lines() {
        if !in_assertion && line.trim().starts_with("const _: () = {") {
            in_assertion = true;
            depth = 0;
            for ch in line.chars() {
                match ch {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
            }
            if depth <= 0 {
                in_assertion = false;
            }
        } else if in_assertion {
            for ch in line.chars() {
                match ch {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
            }
            if depth <= 0 {
                in_assertion = false;
            }
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

fn try_resolve_cyclonedds_source(workspace_root: &Path) -> Result<PathBuf, String> {
    // 1. Environment override (highest priority)
    if let Some(source) = env::var_os("CYCLONEDDS_SRC") {
        let source = PathBuf::from(source);
        if !source.exists() {
            return Err(format!(
                "CYCLONEDDS_SRC does not exist: {}",
                source.display()
            ));
        }
        return Ok(source);
    }

    // 2. cyclonedds-src crate (bundled source — works when published on crates.io)
    let bundled = cyclonedds_src::source_dir();
    if bundled.exists() {
        return Ok(bundled);
    }

    // 3. Workspace vendor directory (local development)
    let vendor = workspace_root.join("vendor/cyclonedds");
    if vendor.exists() {
        Ok(vendor)
    } else {
        Err(
            "CycloneDDS source not found. Set CYCLONEDDS_SRC or ensure vendor/cyclonedds exists."
                .to_string(),
        )
    }
}

fn resolve_cyclonedds_build_dir(source_dir: &Path, out_dir: &Path) -> PathBuf {
    env::var_os("CYCLONEDDS_BUILD")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            out_dir.join("cyclonedds-build").join(
                source_dir
                    .file_name()
                    .unwrap_or_else(|| OsStr::new("cyclonedds")),
            )
        })
}

fn ensure_cyclonedds_build_ready(source_dir: &Path, build_dir: &Path) {
    let enable_security = env::var_os("CARGO_FEATURE_SECURITY").is_some();
    let stamp = build_dir.join(".cargo_features_stamp");
    let stamp_content = format!("security={}\n", enable_security);

    // Check if library exists and feature stamp matches current configuration.
    // If features changed (e.g., security toggled), we must reconfigure/rebuild.
    if find_ddsc_library(build_dir).is_some() {
        if let Ok(existing) = std::fs::read_to_string(&stamp) {
            if existing == stamp_content {
                return;
            }
        }
        // Library exists but stamp doesn't match — skip rebuild, just update stamp
        let _ = std::fs::write(&stamp, &stamp_content);
        return;
    }

    std::fs::create_dir_all(build_dir)
        .unwrap_or_else(|err| panic!("failed to create {}: {err}", build_dir.display()));

    let shared = if cfg!(target_os = "windows") {
        "OFF" // Windows: static linking avoids symbol-export issues with MSVC
    } else if std::env::var_os("CYCLONEDDS_STATIC").is_some() {
        // Opt-in: build estatico (libddsc.a) evita os symlinks de .so versionada,
        // necessario em filesystems sem suporte a symlink (ex.: CIFS/SMB) e produz
        // binario autossuficiente. Requer as libs transitivas (ver emit_link_info).
        "OFF"
    } else {
        "ON"
    };

    let (security_flag, ssl_flag) = if enable_security {
        println!("cargo:warning=DDS Security enabled — ensure OpenSSL is installed");
        ("ON", "ON")
    } else {
        ("OFF", "OFF")
    };

    let mut cmake = Command::new("cmake");
    cmake
        .arg("-S")
        .arg(source_dir)
        .arg("-B")
        .arg(build_dir)
        .arg(format!("-DBUILD_SHARED_LIBS={}", shared))
        // Necessario para linkar a .a estatica em executaveis PIE do Rust.
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .arg("-DBUILD_TESTING=OFF")
        .arg("-DBUILD_IDLC=OFF")
        .arg("-DBUILD_DDSPERF=OFF")
        .arg("-DBUILD_EXAMPLES=OFF")
        .arg("-DENABLE_LTO=OFF")
        .arg(format!("-DENABLE_SECURITY={}", security_flag))
        .arg(format!("-DENABLE_SSL={}", ssl_flag));

    run(&mut cmake, "configure bundled CycloneDDS");
    run(
        Command::new("cmake")
            .arg("--build")
            .arg(build_dir)
            .arg("--target")
            .arg("ddsc")
            .arg("--config")
            .arg("Release"),
        "build bundled CycloneDDS",
    );

    assert!(
        find_ddsc_library(build_dir).is_some(),
        "CycloneDDS build finished but no ddsc library was found under {}",
        build_dir.display()
    );

    // Write feature stamp so we can detect feature changes on subsequent builds.
    std::fs::write(&stamp, stamp_content)
        .unwrap_or_else(|err| panic!("failed to write feature stamp: {err}"));
}

fn find_ddsc_library(build_dir: &Path) -> Option<(PathBuf, &'static str)> {
    let mut stack = vec![build_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let name = path.file_name()?;
            match name.to_str()? {
                "libddsc.dylib" | "libddsc.so" | "ddsc.dll" => {
                    return Some((path.parent()?.to_path_buf(), "dylib"));
                }
                "ddsc.lib" => {
                    // On Windows, ddsc.lib may be an import library (paired with ddsc.dll)
                    // or a static library. If no corresponding dll exists in the same
                    // directory, treat it as static.
                    let dll = path.with_file_name("ddsc.dll");
                    let kind = if dll.exists() { "dylib" } else { "static" };
                    return Some((path.parent()?.to_path_buf(), kind));
                }
                "libddsc.a" => {
                    return Some((path.parent()?.to_path_buf(), "static"));
                }
                _ => {}
            }
        }
    }
    None
}

fn run(command: &mut Command, description: &str) {
    let status = command
        .status()
        .unwrap_or_else(|err| panic!("failed to {description}: {err}"));
    assert!(status.success(), "failed to {description}: {status}");
}

// ── FFI-ABI-008: ABI probe por target ────────────────────────────────────────
//
// Os bindings pré-gerados (macOS/Linux) têm seus asserts de layout de libc
// removidos no build (tipos inexistentes no MSVC). Em troca, este probe
// compila e executa um programa C mínimo contra os MESMOS headers usados no
// link, emitindo os sizeof/offsetof reais do target para um arquivo Rust
// incluído pelo crate — os asserts em `lib.rs` comparam com os tipos dos
// bindings e FALHAM o build em mismatch (corrupção silenciosa vira erro de
// compilação). Em cross-compile (HOST != TARGET) o probe não pode rodar:
// exige-se um snapshot pré-computado em `abi/<target-triple>.rs`.

/// Fonte C do probe. Emite `pub const` Rust com os layouts medidos.
const ABI_PROBE_C: &str = r#"
#include <dds/dds.h>
#include <stddef.h>
#include <stdio.h>
int main(void) {
    printf("pub const PROBE_DDS_SAMPLE_INFO_SIZE: usize = %zu;\n", sizeof(dds_sample_info_t));
    printf("pub const PROBE_OFF_SAMPLE_STATE: usize = %zu;\n", offsetof(dds_sample_info_t, sample_state));
    printf("pub const PROBE_OFF_VIEW_STATE: usize = %zu;\n", offsetof(dds_sample_info_t, view_state));
    printf("pub const PROBE_OFF_INSTANCE_STATE: usize = %zu;\n", offsetof(dds_sample_info_t, instance_state));
    printf("pub const PROBE_OFF_VALID_DATA: usize = %zu;\n", offsetof(dds_sample_info_t, valid_data));
    printf("pub const PROBE_OFF_SOURCE_TIMESTAMP: usize = %zu;\n", offsetof(dds_sample_info_t, source_timestamp));
    printf("pub const PROBE_OFF_INSTANCE_HANDLE: usize = %zu;\n", offsetof(dds_sample_info_t, instance_handle));
    printf("pub const PROBE_DDS_TIME_T_SIZE: usize = %zu;\n", sizeof(dds_time_t));
    printf("pub const PROBE_DDS_DURATION_T_SIZE: usize = %zu;\n", sizeof(dds_duration_t));
    printf("pub const PROBE_DDS_ENTITY_T_SIZE: usize = %zu;\n", sizeof(dds_entity_t));
    printf("pub const PROBE_DDS_RETURN_T_SIZE: usize = %zu;\n", sizeof(dds_return_t));
    printf("pub const PROBE_DDS_INSTANCE_HANDLE_T_SIZE: usize = %zu;\n", sizeof(dds_instance_handle_t));
    printf("pub const PROBE_DDS_ATTACH_T_SIZE: usize = %zu;\n", sizeof(dds_attach_t));
    return 0;
}
"#;

fn run_abi_probe(probe_ctx: Option<&(PathBuf, PathBuf)>, out_dir: &Path) {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let probe_out = out_dir.join("abi_probe.rs");
    let host = env::var("HOST").expect("HOST");
    let target = env::var("TARGET").expect("TARGET");

    // Cross-compile: probe não executa — exige snapshot verificado do target.
    if host != target {
        let snapshot = manifest_dir.join("abi").join(format!("{target}.rs"));
        assert!(
            snapshot.exists(),
            "FFI-ABI-008: cross-compile para {target} sem snapshot ABI em {}. \
             Gere com `cargo build` num host nativo desse target e copie o \
             abi_probe.rs resultante para abi/{target}.rs",
            snapshot.display()
        );
        std::fs::copy(&snapshot, &probe_out).expect("copy ABI snapshot");
        return;
    }

    let Some((cyclonedds_src, cyclonedds_build)) = probe_ctx else {
        // Sem fonte vendored (biblioteca de sistema): não há headers
        // versionados garantidos para o probe — falha explícita e clara.
        panic!(
            "FFI-ABI-008: ABI probe requer a fonte vendored do CycloneDDS \
             (CYCLONEDDS_SRC ou vendor/cyclonedds). Uso de biblioteca de \
             sistema não é uma configuração suportada neste workspace."
        );
    };

    // Cache: re-roda só se o probe.c mudou ou a saída não existe.
    let stamp_path = out_dir.join("abi_probe.stamp");
    let stamp_new = format!("{target}|{:x}", {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        ABI_PROBE_C.hash(&mut h);
        h.finish()
    });
    if probe_out.exists() && std::fs::read_to_string(&stamp_path).is_ok_and(|s| s == stamp_new) {
        return;
    }

    // Include dirs: fonte + headers gerados pelo build nativo.
    let candidates = [
        cyclonedds_src.join("src/core/ddsc/include"),
        cyclonedds_src.join("src/ddsrt/include"),
        cyclonedds_build.join("src/core/include"),
        cyclonedds_build.join("src/ddsrt/include"),
        cyclonedds_build.join("src/core/ddsc/include"),
    ];
    let includes: Vec<PathBuf> = candidates.into_iter().filter(|p| p.exists()).collect();
    assert!(
        includes.len() >= 3,
        "FFI-ABI-008: include dirs do CycloneDDS incompletos para o probe ({includes:?})"
    );
    let includes_cmake = includes
        .iter()
        .map(|p| p.display().to_string().replace('\\', "/"))
        .collect::<Vec<_>>()
        .join(";");

    let proj = out_dir.join("abi_probe_proj");
    std::fs::create_dir_all(&proj).expect("mkdir abi_probe_proj");
    std::fs::write(proj.join("probe.c"), ABI_PROBE_C).expect("write probe.c");
    std::fs::write(
        proj.join("CMakeLists.txt"),
        format!(
            "cmake_minimum_required(VERSION 3.16)\nproject(abi_probe C)\n\
             add_executable(abi_probe probe.c)\n\
             target_include_directories(abi_probe PRIVATE {includes_cmake})\n"
        ),
    )
    .expect("write probe CMakeLists");

    let probe_build = proj.join("build");
    run(
        Command::new("cmake")
            .arg("-S")
            .arg(&proj)
            .arg("-B")
            .arg(&probe_build),
        "configure ABI probe",
    );
    run(
        Command::new("cmake")
            .arg("--build")
            .arg(&probe_build)
            .arg("--config")
            .arg("Release"),
        "build ABI probe",
    );

    // Multi-config (MSVC) grava em Release/; single-config na raiz do build.
    let exe_candidates = [
        probe_build.join("Release").join("abi_probe.exe"),
        probe_build.join("Release").join("abi_probe"),
        probe_build.join("abi_probe.exe"),
        probe_build.join("abi_probe"),
    ];
    let exe = exe_candidates
        .iter()
        .find(|p| p.exists())
        .unwrap_or_else(|| {
            panic!("FFI-ABI-008: binário do probe não encontrado em {probe_build:?}")
        });
    let output = Command::new(exe)
        .output()
        .unwrap_or_else(|e| panic!("FFI-ABI-008: falha ao executar probe: {e}"));
    assert!(
        output.status.success(),
        "FFI-ABI-008: probe retornou {}",
        output.status
    );
    let stdout = String::from_utf8(output.stdout).expect("probe output utf8");
    assert!(
        stdout.contains("PROBE_DDS_SAMPLE_INFO_SIZE"),
        "FFI-ABI-008: saída do probe inesperada: {stdout}"
    );
    std::fs::write(&probe_out, &stdout).expect("write abi_probe.rs");
    std::fs::write(&stamp_path, stamp_new).expect("write probe stamp");
    println!(
        "cargo:warning=ABI probe OK para {target} ({} bytes de checks)",
        stdout.len()
    );
}
