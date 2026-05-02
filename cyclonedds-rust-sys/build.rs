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
    let lib_found = if let Ok(cyclonedds_src) = try_resolve_cyclonedds_source(workspace_root) {
        let cyclonedds_build = resolve_cyclonedds_build_dir(&cyclonedds_src, &out_dir);

        // Check if already built, otherwise try to build (requires cmake)
        if find_ddsc_library(&cyclonedds_build).is_some() {
            if let Some((lib_dir, link_kind)) = find_ddsc_library(&cyclonedds_build) {
                emit_link_info(&lib_dir, link_kind);
                true
            } else {
                false
            }
        } else if which_cmake().is_some() {
            ensure_cyclonedds_build_ready(&cyclonedds_src, &cyclonedds_build);
            if let Some((lib_dir, link_kind)) = find_ddsc_library(&cyclonedds_build) {
                emit_link_info(&lib_dir, link_kind);
                true
            } else {
                false
            }
        } else {
            // cmake not available — fall back to system library search
            if let Some((lib_dir, link_kind)) = find_system_ddsc_library() {
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
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=build.rs");

    // Use prebuilt bindings (generated on macOS/Linux where bindgen works correctly
    // with the CycloneDDS headers). We skip bindgen entirely because clang on
    // Windows/MSVC fails to resolve many types that are correctly resolved on
    // other platforms.
    let bindings_path = out_dir.join("bindings.rs");

    if prebuilt.exists() {
        // Strip static assertions that fail due to platform differences
        let content = std::fs::read_to_string(&prebuilt).expect("couldn't read prebuilt bindings");
        let stripped = strip_static_assertions_from_str(&content);
        std::fs::write(&bindings_path, stripped).expect("couldn't write bindings");
    } else {
        panic!(
            "No prebuilt bindings found at {}. Run bindgen on macOS/Linux first.",
            prebuilt.display()
        );
    }
}

fn emit_link_info(lib_dir: &Path, link_kind: &'static str) {
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib={link_kind}=ddsc");
    if link_kind == "dylib" {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
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
    }

    std::fs::create_dir_all(build_dir)
        .unwrap_or_else(|err| panic!("failed to create {}: {err}", build_dir.display()));

    let shared = if cfg!(target_os = "windows") {
        "OFF" // Windows: static linking avoids symbol-export issues with MSVC
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
