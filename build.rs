fn main() {
    // Vendored OpenSSL MSVC objects do not ship their compiler PDB. The missing
    // native debug symbols are harmless; keep all other linker diagnostics.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg=/IGNORE:4099");
    }
}
