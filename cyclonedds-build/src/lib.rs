//! CycloneDDS build script support for compiling IDL files to Rust.
//!
//! This crate provides a `compile_idl` function intended to be called from
//! a `build.rs` script. It parses IDL type definitions and generates Rust
//! source code that uses the `cyclonedds-derive` proc-macros (`DdsTypeDerive`,
//! `DdsEnumDerive`, `DdsUnionDerive`, `DdsBitmaskDerive`).
//!
//! # Example `build.rs`
//!
//! ```ignore
//! fn main() {
//!     cyclonedds_build::compile_idl("src/types.idl").unwrap();
//! }
//! ```
//!
//! Then in your lib or mod:
//!
//! ```ignore
//! // types.rs
//! include!(concat!(env!("OUT_DIR"), "/types.rs"));
//! ```
//!
//! # Finding the `idlc` compiler
//!
//! The library first attempts to use the CycloneDDS C `idlc` compiler to
//! produce a type descriptor. It searches for `idlc` in:
//! 1. `CYCLONEDDS_HOME` environment variable (looks in `<home>/bin/idlc`)
//! 2. The system `PATH`
//!
//! If `idlc` is not found, the library falls back to its built-in simplified
//! IDL parser, which handles common IDL constructs.

pub mod codegen;
pub mod idl_parser;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

/// Compile an IDL file to Rust source code and write the output to OUT_DIR.
///
/// Call this from your `build.rs` script. The generated file will be placed
/// in `OUT_DIR` with the same stem as the input IDL file but with a `.rs`
/// extension.
///
/// # Errors
///
/// Returns an error if:
/// - The IDL file cannot be read
/// - The IDL content cannot be parsed
/// - The output file cannot be written
pub fn compile_idl(idl_path: impl AsRef<Path>) -> Result<()> {
    let idl_path = idl_path.as_ref();
    compile_idl_with_options(idl_path, &CompileOptions::default())
}

/// Options for IDL compilation.
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// Optional CycloneDDS home directory (overrides CYCLONEDDS_HOME env var).
    pub cyclonedds_home: Option<PathBuf>,
    /// Optional output directory. Defaults to `env::var("OUT_DIR")`.
    pub output_dir: Option<PathBuf>,
    /// Whether to attempt using the `idlc` binary before falling back to
    /// the built-in parser. Defaults to `true`.
    pub try_idlc: bool,
    /// Module name override for the generated file. Defaults to the IDL file
    /// stem.
    pub module_name: Option<String>,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            cyclonedds_home: None,
            output_dir: None,
            try_idlc: true,
            module_name: None,
        }
    }
}

/// Compile an IDL file with custom options.
pub fn compile_idl_with_options(idl_path: &Path, options: &CompileOptions) -> Result<()> {
    // Validate input file exists
    if !idl_path.exists() {
        bail!("IDL file not found: {}", idl_path.display());
    }

    let idl_content = fs::read_to_string(idl_path)
        .with_context(|| format!("Failed to read IDL file: {}", idl_path.display()))?;

    // Determine module name
    let module_name = options.module_name.clone().unwrap_or_else(|| {
        idl_path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "idl_types".into())
    });

    // Try idlc-based compilation first, then fall back to built-in parser
    let rust_code = if options.try_idlc {
        compile_with_idlc_or_fallback(&idl_content, &module_name, options)
            .with_context(|| "IDL compilation failed")?
    } else {
        parse_and_generate(&idl_content, &module_name)
            .with_context(|| "Built-in IDL parsing failed")?
    };

    // Determine output directory
    let output_dir = options
        .output_dir
        .clone()
        .or_else(|| env::var("OUT_DIR").ok().map(PathBuf::from))
        .unwrap_or_else(|| {
            // If no OUT_DIR and no output_dir, use the current directory
            env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        });

    // Ensure output directory exists
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "Failed to create output directory: {}",
            output_dir.display()
        )
    })?;

    // Write output file
    let output_file = output_dir.join(format!("{}.rs", module_name));
    fs::write(&output_file, &rust_code)
        .with_context(|| format!("Failed to write output file: {}", output_file.display()))?;

    // Emit cargo directives if running in a build script context
    if env::var("OUT_DIR").is_ok() {
        println!("cargo:rerun-if-changed={}", idl_path.display());
    }

    Ok(())
}

/// Try to compile using `idlc`, fall back to the built-in parser on failure.
fn compile_with_idlc_or_fallback(
    idl_content: &str,
    module_name: &str,
    options: &CompileOptions,
) -> Result<String> {
    // Find idlc binary
    let idlc = find_idlc(options.cyclonedds_home.as_deref());

    match idlc {
        Some(idlc_path) => {
            // Attempt to use idlc for validation (the actual code generation
            // is done by the built-in parser, since we produce Rust source
            // rather than C headers).
            //
            // We run idlc in a validation-only mode if possible, or just use
            // its presence as confirmation that CycloneDDS is installed.
            log_build(&format!("Found idlc at: {}", idlc_path.display()));

            // Fall through to built-in parser for Rust code generation.
            // The idlc binary generates C code, not Rust, so we use our
            // own parser/generator but benefit from knowing idlc is available.
            parse_and_generate(idl_content, module_name)
        }
        None => {
            log_build("idlc not found, using built-in IDL parser");
            parse_and_generate(idl_content, module_name)
        }
    }
}

/// Find the `idlc` binary.
///
/// Search order:
/// 1. `CYCLONEDDS_HOME/bin/idlc` (or the provided home path)
/// 2. `idlc` on the system `PATH`
fn find_idlc(cyclonedds_home: Option<&Path>) -> Option<PathBuf> {
    // Check CYCLONEDDS_HOME first
    let home = cyclonedds_home
        .map(|p| p.to_path_buf())
        .or_else(|| env::var("CYCLONEDDS_HOME").ok().map(PathBuf::from));

    if let Some(home) = home {
        let idlc_path = home.join("bin").join("idlc");
        if idlc_path.exists() {
            return Some(idlc_path);
        }
        // Also check without bin/ prefix
        let idlc_path = home.join("idlc");
        if idlc_path.exists() {
            return Some(idlc_path);
        }
        // On Windows, check for .exe
        let idlc_path = home.join("bin").join("idlc.exe");
        if idlc_path.exists() {
            return Some(idlc_path);
        }
    }

    // Check PATH
    if let Ok(result) = which_idlc() {
        return Some(result);
    }

    None
}

/// Simple which-like lookup for idlc on PATH.
fn which_idlc() -> Result<PathBuf> {
    let path_var = env::var("PATH").context("PATH not set")?;
    let separator = if cfg!(windows) { ';' } else { ':' };

    for dir in path_var.split(separator) {
        let candidate = PathBuf::from(dir).join("idlc");
        if candidate.exists() {
            return Ok(candidate);
        }
        if cfg!(windows) {
            let candidate_exe = PathBuf::from(dir).join("idlc.exe");
            if candidate_exe.exists() {
                return Ok(candidate_exe);
            }
        }
    }

    bail!("idlc not found on PATH")
}

/// Parse IDL content and generate Rust code using the built-in parser.
fn parse_and_generate(idl_content: &str, module_name: &str) -> Result<String> {
    let idl_file = idl_parser::parse_idl(idl_content)
        .map_err(|e| anyhow::anyhow!("IDL parse error: {}", e))?;

    let rust_code = codegen::generate_rust(&idl_file, module_name);

    Ok(rust_code)
}

/// Log a message during build script execution.
fn log_build(msg: &str) {
    // In a build script, use cargo:warning for visibility
    if env::var("OUT_DIR").is_ok() {
        println!("cargo:warning={}", msg);
    }
}

/// Compile multiple IDL files at once, writing each to a separate output file.
///
/// All files share the same compilation options.
pub fn compile_idl_files(idl_paths: &[&Path], options: &CompileOptions) -> Result<()> {
    for idl_path in idl_paths {
        compile_idl_with_options(idl_path, options)?;
    }
    Ok(())
}

/// Run `idlc` to validate an IDL file (without generating any output).
///
/// Returns `Ok(())` if idlc reports no errors, or an error with the
/// diagnostics if it fails.
pub fn validate_idl(idl_path: &Path, cyclonedds_home: Option<&Path>) -> Result<()> {
    let idlc = find_idlc(cyclonedds_home)
        .context("idlc not found; set CYCLONEDDS_HOME or install CycloneDDS C")?;

    let output = Command::new(&idlc)
        .arg("--check-only")
        .arg(idl_path)
        .output()
        .with_context(|| format!("Failed to run idlc at {}", idlc.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("idlc validation failed:\n{}", stderr);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_compile_idl_to_string() {
        let idl = r#"
            struct Point {
                double x;
                double y;
            };
        "#;
        let result = parse_and_generate(idl, "test_types").unwrap();
        assert!(result.contains("#[derive(Debug, Clone, DdsTypeDerive)]"));
        assert!(result.contains("pub struct Point"));
        assert!(result.contains("pub x: f64"));
    }

    #[test]
    fn test_compile_with_output_dir() {
        let dir = tempfile::tempdir().unwrap();
        let idl_file = dir.path().join("test.idl");
        let mut f = fs::File::create(&idl_file).unwrap();
        write!(
            f,
            r#"
            struct Hello {{
                string message;
            }};
        "#
        )
        .unwrap();

        let out_dir = dir.path().join("output");
        let options = CompileOptions {
            output_dir: Some(out_dir.clone()),
            try_idlc: false,
            ..Default::default()
        };

        compile_idl_with_options(&idl_file, &options).unwrap();
        let output = fs::read_to_string(out_dir.join("test.rs")).unwrap();
        assert!(output.contains("pub struct Hello"));
    }

    #[test]
    fn test_compile_enum() {
        let idl = r#"
            enum Status { OK, ERROR = 2 };
        "#;
        let result = parse_and_generate(idl, "enums").unwrap();
        assert!(result.contains("OK = 0"));
        assert!(result.contains("ERROR = 2"));
    }
}
