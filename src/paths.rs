use std::path::PathBuf;

pub(crate) fn download_root() -> PathBuf {
    match std::env::var_os("HOME") {
        Some(home) => PathBuf::from(home)
        .join("reims-gui-downloads"),

        None => PathBuf::from("."),
    }
}

pub(crate) fn macos_download_path(version: &str) -> PathBuf {
    download_root()
    .join("macos")
    .join(version)
}

pub(crate) fn osx_kvm_path() -> PathBuf {
    download_root().join("OSX-KVM")
}

pub(crate) fn reims_vgpu_path() -> PathBuf {
    download_root().join("reims-vgpu")
}

pub(crate) fn osx_kvm_installed() -> bool {
    osx_kvm_path()
    .join("fetch-macOS-v2.py")
    .is_file()
}

pub(crate) fn reims_vgpu_installed() -> bool {
    reims_vgpu_path()
    .join("vm")
    .join("boot-x86.sh")
    .is_file()
}
