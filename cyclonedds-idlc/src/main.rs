//! CLI tool for compiling IDL files to Rust source code for CycloneDDS.
//!
//! Usage:
//!   cyclonedds-idlc --input types.idl --output-dir src/dds_types/
//!   cyclonedds-idlc --input types.idl --output-dir src/dds_types/ --cyclonedds-home /path/to/cyclonedds

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use cyclonedds_build::{compile_idl_with_options, CompileOptions};

#[derive(Parser, Debug)]
#[command(
    name = "cyclonedds-idlc",
    version,
    about = "Compile OMG IDL files to Rust source code for CycloneDDS",
    long_about = "Parses IDL type definitions and generates Rust source code that uses \
                  cyclonedds-derive proc-macros (DdsTypeDerive, DdsEnumDerive, DdsUnionDerive, DdsBitmaskDerive)."
)]
struct Args {
    /// Input IDL file to compile.
    #[arg(short, long)]
    input: PathBuf,

    /// Output directory for generated Rust source files.
    #[arg(short, long)]
    output_dir: Option<PathBuf>,

    /// Path to CycloneDDS installation (containing bin/idlc).
    #[arg(long)]
    cyclonedds_home: Option<PathBuf>,

    /// Module name override for the generated code.
    /// Defaults to the input file stem.
    #[arg(short, long)]
    module_name: Option<String>,

    /// Skip attempting to use the idlc binary.
    #[arg(long)]
    no_idlc: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Validate input file
    if !args.input.exists() {
        anyhow::bail!(
            "Input IDL file not found: {}",
            args.input.display()
        );
    }

    // Default output directory: current directory
    let output_dir = args.output_dir.unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    });

    let options = CompileOptions {
        cyclonedds_home: args.cyclonedds_home,
        output_dir: Some(output_dir.clone()),
        try_idlc: !args.no_idlc,
        module_name: args.module_name,
    };

    compile_idl_with_options(&args.input, &options).with_context(|| {
        format!(
            "Failed to compile IDL file: {}",
            args.input.display()
        )
    })?;

    println!(
        "Successfully compiled {} -> {}/",
        args.input.display(),
        output_dir.display()
    );

    Ok(())
}
