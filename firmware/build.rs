use std::{env, fs, path::{Path, PathBuf}};

fn main() {
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::copy("memory.x", out.join("memory.x")).unwrap();
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rustc-link-arg=-Tlink.x");
    // Without this, lld maps the ELF headers at 0x08000000, inside the bootloader.
    println!("cargo:rustc-link-arg=--nmagic");
    println!("cargo:rerun-if-changed=memory.x");
    voice_clips(&out);
}

/// The voice probe's clips: `clips.rs` from the folder `VOICE_CLIPS` names, which
/// `tools/pack/voiceclips.py` writes, or no clips, so every other build needs no clips.
fn voice_clips(out: &Path) {
    println!("cargo:rerun-if-env-changed=VOICE_CLIPS");
    let clips = match env::var_os("VOICE_CLIPS") {
        Some(folder) => {
            let path = PathBuf::from(folder).join("clips.rs");
            println!("cargo:rerun-if-changed={}", path.display());
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        }
        None => "const VOICE: &str = \"NO CLIPS: SEE tools/pack/voiceclips.py\";\nconst CLIPS: &[Clip] = &[];\n".into(),
    };
    fs::write(out.join("clips.rs"), clips).unwrap();
}
