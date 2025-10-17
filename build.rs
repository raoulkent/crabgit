// Man page generation temporarily disabled
// These imports would be needed for future man page generation

fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=build.rs");

    // Skip man page generation for now to avoid complex module resolution
    // This can be added back with proper module structure later

    Ok(())
}
