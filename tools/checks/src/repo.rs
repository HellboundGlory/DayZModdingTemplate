use std::path::{Path, PathBuf};

// Fixed at build time, so a command works from any folder.
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("tools/checks sits two folders below the repo root")
        .to_path_buf()
}
