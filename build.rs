use std::env;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // Reserve an 8 MiB main-thread stack on Windows (MSVC defaults to 1 MiB,
        // MinGW to 2 MiB), matching Linux/macOS so QuickJS-NG's 4 MiB stack guard
        // triggers cleanly before hitting the OS stack guard page during Angular boot.
        if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
            println!("cargo:rustc-link-arg-bins=/STACK:8388608");
            println!("cargo:rustc-link-arg-tests=/STACK:8388608");
        } else {
            println!("cargo:rustc-link-arg-bins=-Wl,--stack,8388608");
            println!("cargo:rustc-link-arg-tests=-Wl,--stack,8388608");
        }
    }
}
