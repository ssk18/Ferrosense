//! FerroSense core: framing, validation, and parsing of the sensor protocol.
//!
//! This crate is **pure Rust** — no Android, no JNI. It builds, tests, fuzzes,
//! and benchmarks on any host at native speed. The `//!` above is an *inner*
//! doc comment: it documents the thing it's inside of (here, the whole crate).

/// A temporary smoke-test target so we can prove the toolchain end-to-end.
/// We'll replace this with real protocol types in the next session.
///
/// `&'static str` = a string slice that lives for the entire program.
/// (That `'static` is a *lifetime* — we'll unpack lifetimes properly later;
/// for now read it as "this borrow is always valid.")
pub fn version() -> &'static str {
    // `env!` is a compile-time macro: Cargo injects CARGO_PKG_VERSION
    // (your "0.1.0") as a string literal baked into the binary.
    env!("CARGO_PKG_VERSION")
}

// `#[cfg(test)]` = conditional compilation. This module is compiled ONLY
// during `cargo test`, so tests add zero bytes to the shipped library.
#[cfg(test)]
mod tests {
    use super::*; // pull the parent module's items (like `version`) into scope

    #[test] // marks a function the test harness should run
    fn version_is_reported() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }
}
