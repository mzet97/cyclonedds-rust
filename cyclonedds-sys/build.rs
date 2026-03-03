// Build script for cyclonedds-sys
// Currently using manual bindings - no build-time code generation

fn main() {
    // This build script is intentionally minimal
    // The FFI bindings are manually defined in src/lib.rs
    //
    // To link with CycloneDDS:
    // - Windows: Ensure ddsc.dll is in PATH or next to executable
    // - Linux: Install libddsc-dev and link with -lddsc

    println!("cargo:rerun-if-changed=build.rs");
}
