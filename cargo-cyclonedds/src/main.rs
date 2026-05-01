//! Cargo plugin for generating Rust types from OMG IDL files for CycloneDDS.
//!
//! Install with:
//!   cargo install cargo-cyclonedds
//!
//! Usage:
//!   cargo cyclonedds generate HelloWorld.idl
//!   cargo cyclonedds generate types.idl --output-dir src/dds_types/

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use cyclonedds_build::{compile_idl_with_options, CompileOptions};

#[derive(Parser, Debug)]
#[command(
    name = "cargo-cyclonedds",
    version,
    about = "Cargo plugin for CycloneDDS IDL code generation",
    long_about = "Generate Rust source code from OMG IDL files for use with cyclonedds."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate Rust code from an IDL file.
    Generate {
        /// Input IDL file to compile.
        idl_file: PathBuf,

        /// Output directory for the generated Rust source file.
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// Path to CycloneDDS installation (containing bin/idlc).
        #[arg(long)]
        cyclonedds_home: Option<PathBuf>,

        /// Module name override for the generated code.
        #[arg(short, long)]
        module_name: Option<String>,

        /// Skip attempting to use the idlc binary.
        #[arg(long)]
        no_idlc: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            idl_file,
            output_dir,
            cyclonedds_home,
            module_name,
            no_idlc,
        } => {
            if !idl_file.exists() {
                anyhow::bail!("IDL file not found: {}", idl_file.display());
            }

            let out_dir = output_dir.unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            });

            std::fs::create_dir_all(&out_dir)
                .with_context(|| format!("creating output directory {}", out_dir.display()))?;

            let options = CompileOptions {
                cyclonedds_home,
                output_dir: Some(out_dir.clone()),
                try_idlc: !no_idlc,
                module_name,
            };

            compile_idl_with_options(&idl_file, &options)
                .with_context(|| format!("compiling {}", idl_file.display()))?;

            let stem = idl_file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            let out_file = out_dir.join(format!("{}.rs", stem));

            println!("Generated: {}", out_file.display());
        }
    }

    Ok(())
}
