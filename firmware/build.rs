use std::{env, fs, path::PathBuf};

fn main() {
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::copy("memory.x", out.join("memory.x")).unwrap();
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rustc-link-arg=-Tlink.x");
    // Without this, lld maps the ELF headers at 0x08000000, inside the bootloader.
    println!("cargo:rustc-link-arg=--nmagic");
    println!("cargo:rerun-if-changed=memory.x");
}
