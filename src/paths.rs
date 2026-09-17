use std::path::PathBuf;

pub(crate) fn download_root() -> PathBuf {
    match std::env::var_os("HOME") {
        Some(home) => PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("reims-vgpu"),

        None => PathBuf::from("."),
    }
}

pub(crate) fn macos_download_path(version: &str) -> PathBuf {
    download_root()
    .join("downloads")
    .join(version)
}
