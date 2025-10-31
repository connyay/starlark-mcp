fn main() {
    // Get version from Cargo.toml and make it available at compile time
    let version = env!("CARGO_PKG_VERSION");
    println!("cargo:rustc-env=MCP_STAR_VERSION={}", version);

    // Rerun if Cargo.toml changes
    println!("cargo:rerun-if-changed=Cargo.toml");
}
