use eframe::egui;
use std::process::{Command, Stdio};

use crate::app::ReimsVgpuApp;
use crate::paths::{download_root, macos_download_path};

// Function to download macos recovery
impl ReimsVgpuApp {

    pub(crate) fn download_macos(&mut self) {
        if self.download_process.is_some() {
            return;
        }

        let osx_kvm = download_root().join("OSX-KVM");
        let fetch_macos = osx_kvm.join("fetch-macOS-v2.py");

        if !fetch_macos.exists() {
            self.download_status =
            format!("fetch-macOS-v2.py not found: {}", fetch_macos.display());
            return;
        }

        let (board_id, mlb, os_type) = match self.macOS_version.as_str() {
            "macOS 11 Big Sur" => (
                "Mac-2BD1B31983FE1663",
                "00000000000000000",
                "default",
            ),

            "macOS 12 Monterey" => (
                "Mac-B809C3757DA9BB8D",
                "00000000000000000",
                "latest",
            ),

            "macOS 13 Ventura" => (
                "Mac-4B682C642B45593E",
                "00000000000000000",
                "latest",
            ),

            "macOS 14 Sonoma" => (
                "Mac-827FAC58A8FDFA22",
                "00000000000000000",
                "default",
            ),

            "macOS 15 Sequoia" => (
                "Mac-7BA5B2D9E42DDD94",
                "00000000000000000",
                "default",
            ),

            "macOS 26 Tahoe" => (
                "Mac-CFF7D910A743CAAF",
                "00000000000000000",
                "latest",
            ),

            _ => {
                self.download_status =
                "Unknown macOS version.".to_string();
                return;
            }
        };

        let output_dir = download_root()
        .join("macos")
        .join(&self.macOS_version);

        if let Err(error) = std::fs::create_dir_all(&output_dir) {
            self.download_status =
            format!("Could not create download directory: {}", error);
            return;
        }

        let log_path = output_dir.join("download.log");

        let log_file = match std::fs::File::create(&log_path) {
            Ok(file) => file,

            Err(error) => {
                self.download_status =
                format!("Could not create downloader log: {}", error);
                return;
            }
        };

        let stderr_file = match log_file.try_clone() {
            Ok(file) => file,

            Err(error) => {
                self.download_status =
                format!("Could not prepare downloader log: {}", error);
                return;
            }
        };

        self.download_product.clear();
        self.download_log_path = Some(log_path);

        self.download_status =
        format!("Downloading {}...", self.macOS_version);

        match Command::new("python3")
        .arg(&fetch_macos)
        .arg("--action")
        .arg("download")
        .arg("--board-id")
        .arg(board_id)
        .arg("--mlb")
        .arg(mlb)
        .arg("--os-type")
        .arg(os_type)
        .arg("--outdir")
        .arg(&output_dir)
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()
        {
            Ok(process) => {
                self.download_process = Some(process);
            }

            Err(error) => {
                self.download_status =
                format!("Could not start fetch-macOS-v2.py: {}", error);
                self.download_log_path = None;
            }
        }
    }

    // Update download status

    pub(crate) fn update_download(&mut self, ctx: &egui::Context) {
        // Only inspect the downloader log while the downloader
        // process is actually running.
        if self.download_process.is_some() {
            if let Some(path) = &self.download_log_path {
                if let Ok(contents) = std::fs::read_to_string(path) {
                    for line in contents.lines() {
                        let line = line.trim();

                        if let Some(product) = line.strip_prefix("Downloading ") {
                            let product = product
                            .trim()
                            .trim_end_matches('.');

                            if !product.is_empty() {
                                self.download_product = product.to_string();

                                self.download_status = format!(
                                    "Downloading {} — Apple recovery {}...",
                                    self.macOS_version,
                                    self.download_product
                                );
                            }
                        }
                    }
                }
            }
        }

        let Some(process) = &mut self.download_process else {
            return;
        };

        match process.try_wait() {
            Ok(Some(status)) => {
                self.download_process = None;

                if status.success() {
                    if self.download_product.is_empty() {
                        self.download_status =
                        "Download complete!".to_string();
                    } else {
                        self.download_status = format!(
                            "Download complete — Apple recovery {}.",
                            self.download_product
                        );
                    }
                } else {
                    self.download_status =
                    "Download failed.".to_string();
                }
            }

            Ok(None) => {
                ctx.request_repaint();
            }

            Err(error) => {
                self.download_status =
                format!("Failed to check download process: {}", error);

                self.download_process = None;
            }
        }
    }

    // Create virtual macos disk

    pub(crate) fn create_virtual_disk(&mut self) {
        if self.disk_created {
            return;
        }

        let home = match std::env::var_os("HOME") {
            Some(home) => std::path::PathBuf::from(home),
            None => {
                self.disk_status = "Could not determine home directory.".to_string();
                return;
            }
        };

        let osx_kvm = download_root().join("OSX-KVM");
        let disk_path = osx_kvm.join("mac_hdd_ng.img");

        if let Err(error) = std::fs::create_dir_all(&osx_kvm) {
            self.disk_status = format!("Could not create OSX-KVM directory: {}", error);
            return;
        }

        if disk_path.exists() {
            self.disk_status = format!("Virtual disk already exists: {}", disk_path.display());
            self.disk_created = true;
            return;
        }

        self.disk_status = "Creating macOS virtual disk...".to_string();

        let size = format!("{}G", self.disk_size_gb);

        match Command::new("qemu-img")
        .arg("create")
        .arg("-f")
        .arg("qcow2")
        .arg(&disk_path)
        .arg(&size)
        .output()
        {
            Ok(output) => {
                if output.status.success() {
                    self.disk_status = format!("Virtual disk created: {} GB", self.disk_size_gb);
                    self.disk_created = true;
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);

                    self.disk_status = format!("Failed to create virtual disk: {}", error.trim());
                }
            }

            Err(error) => {
                self.disk_status = format!("Could not run qemu-img: {}", error);
            }
        }
    }

    // run the vm for installing macos using the recovery

    pub(crate) fn run_macos_installer(&mut self) {
        if self.installer_process.is_some() {
            return;
        }

        let home = match std::env::var_os("HOME") {
            Some(home) => std::path::PathBuf::from(home),
            None => {
                self.installer_status =
                "Could not determine home directory.".to_string();
                return;
            }
        };

        let osx_kvm = download_root().join("OSX-KVM");

        let recovery_dir = download_root()
        .join("macos")
        .join(&self.macOS_version);

        let downloaded_dmg = recovery_dir.join("BaseSystem.dmg");
        let osx_kvm_dmg = osx_kvm.join("BaseSystem.dmg");
        let base_system_img = osx_kvm.join("BaseSystem.img");
        let mac_hdd = osx_kvm.join("mac_hdd_ng.img");
        let boot_script = osx_kvm.join("OpenCore-Boot.sh");

        // Check OSX-KVM.
        if !osx_kvm.exists() {
            self.installer_status =
            "OSX-KVM is not installed. Download it first.".to_string();
            return;
        }

        // Check that macOS recovery was downloaded.
        if !downloaded_dmg.exists() {
            self.installer_status = format!(
                "BaseSystem.dmg has not been downloaded for {}.",
                self.macOS_version
            );
            return;
        }

        // Check the OSX-KVM launcher.
        if !boot_script.exists() {
            self.installer_status =
            format!("OpenCore-Boot.sh not found: {}", boot_script.display());
            return;
        }

        // The virtual disk must exist.
        if !mac_hdd.exists() {
            self.installer_status =
            "mac_hdd_ng.img does not exist. Create the macOS disk first."
            .to_string();
            self.disk_created = false;
            return;
        }

        self.installer_status = format!(
            "Preparing {} recovery image...",
            self.macOS_version
        );

        // Always replace the old recovery DMG with the selected version.
        if let Err(error) = std::fs::copy(&downloaded_dmg, &osx_kvm_dmg) {
            self.installer_status =
            format!("Could not copy BaseSystem.dmg: {}", error);
            return;
        }

        // Always rebuild BaseSystem.img from the newly selected DMG.
        if base_system_img.exists() {
            if let Err(error) = std::fs::remove_file(&base_system_img) {
                self.installer_status =
                format!("Could not remove old BaseSystem.img: {}", error);
                return;
            }
        }

        self.installer_status =
        "Converting BaseSystem.dmg to BaseSystem.img...".to_string();

        match Command::new("dmg2img")
        .arg(&osx_kvm_dmg)
        .arg(&base_system_img)
        .output()
        {
            Ok(output) if output.status.success() => {}

            Ok(output) => {
                let error = String::from_utf8_lossy(&output.stderr);

                self.installer_status =
                format!("dmg2img failed: {}", error.trim());
                return;
            }

            Err(error) => {
                self.installer_status =
                format!("Could not run dmg2img: {}", error);
                return;
            }
        }

        self.installer_status =
        format!("Starting {} installer...", self.macOS_version);

        match Command::new("bash")
        .arg(&boot_script)
        .current_dir(&osx_kvm)
        .spawn()
        {
            Ok(process) => {
                self.installer_process = Some(process);
                self.installer_status =
                "macOS installer is running.".to_string();
            }

            Err(error) => {
                self.installer_status =
                format!("Could not start OSX-KVM: {}", error);
            }
        }
    }

    // update on install vm

    pub(crate) fn update_installer(&mut self, ctx: &egui::Context) {
        let Some(process) = &mut self.installer_process else {
            return;
        };

        match process.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    self.installer_status = "Installer VM closed successfully.".to_string();
                } else {
                    self.installer_status =
                    format!("Installer VM exited with code {:?}.", status.code());
                }

                self.installer_process = None;
            }

            Ok(None) => {
                // QEMU is still running.
                ctx.request_repaint();
            }

            Err(error) => {
                self.installer_status = format!("Failed to check installer VM: {}", error);
                self.installer_process = None;
            }
        }
    }

    // download osx kvm repo

    pub(crate) fn download_osx_kvm(&mut self) {
            if self.osx_kvm_process.is_some() {
            return;
        }

        let root = download_root();

        if let Err(error) = std::fs::create_dir_all(&root) {
            self.osx_kvm_status = format!(
                "Could not create download directory: {}",
                error
            );
            return;
        }

        let osx_kvm = root.join("OSX-KVM");

        let repo_exists =
        osx_kvm.join(".git").exists() &&
        osx_kvm.join("OpenCore").exists();

        if repo_exists {
            self.osx_kvm_status =
            "OSX-KVM is already installed.".to_string();
            return;
        }

        self.osx_kvm_status =
        "Downloading OSX-KVM repository...".to_string();

        if osx_kvm.exists() {
            if let Err(error) = std::fs::remove_dir_all(&osx_kvm) {
                self.osx_kvm_status = format!(
                    "Could not remove incomplete OSX-KVM directory: {}",
                    error
                );
                return;
            }
        }

        let temp_path = root.join("OSX-KVM-clone");

        if temp_path.exists() {
            if let Err(error) = std::fs::remove_dir_all(&temp_path) {
                self.osx_kvm_status = format!(
                    "Could not remove old OSX-KVM clone: {}",
                    error
                );
                return;
            }
        }

        match Command::new("git")
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg("--recursive")
        .arg("https://github.com/kholia/OSX-KVM.git")
        .arg(&temp_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        {
            Ok(process) => {
                self.osx_kvm_process = Some(process);
            }

            Err(error) => {
                self.osx_kvm_status =
                format!("Could not start git: {}", error);
            }
        }
    }

    // update on osx kvm download status

    pub(crate) fn update_osx_kvm(&mut self, ctx: &egui::Context) {
        let Some(process) = &mut self.osx_kvm_process else {
            return;
        };

        match process.try_wait() {
            Ok(Some(status)) => {
                self.osx_kvm_process = None;

                if !status.success() {
                    self.osx_kvm_status = "OSX-KVM download failed.".to_string();
                    return;
                }

                let home = match std::env::var_os("HOME") {
                    Some(home) => std::path::PathBuf::from(home),
                    None => {
                        self.osx_kvm_status = "Could not determine home directory.".to_string();
                        return;
                    }
                };

                let osx_kvm = download_root().join("OSX-KVM");
                let temp_path = download_root().join("OSX-KVM-clone");

                if !temp_path.join(".git").exists() {
                    self.osx_kvm_status =
                    "OSX-KVM clone completed but repository files are missing.".to_string();
                    return;
                }

                // The destination directory currently contains the disk we
                // created earlier. Preserve that disk while moving the repo.
                let disk_path = osx_kvm.join("mac_hdd_ng.img");

                if disk_path.exists() {
                    let temp_disk = temp_path.join("mac_hdd_ng.img");

                    if let Err(error) = std::fs::rename(&disk_path, &temp_disk) {
                        self.osx_kvm_status = format!("Could not preserve macOS disk: {}", error);
                        return;
                    }
                }

                // Remove the now-empty/incomplete destination.
                if osx_kvm.exists() {
                    if let Err(error) = std::fs::remove_dir_all(&osx_kvm) {
                        self.osx_kvm_status =
                        format!("Could not replace incomplete OSX-KVM directory: {}", error);
                        return;
                    }
                }

                if let Err(error) = std::fs::rename(&temp_path, &osx_kvm) {
                    self.osx_kvm_status =
                    format!("Could not install OSX-KVM repository: {}", error);
                    return;
                }

                self.osx_kvm_status = "OSX-KVM downloaded successfully.".to_string();
            }

            Ok(None) => {
                ctx.request_repaint();
            }

            Err(error) => {
                self.osx_kvm_status = format!("Failed to check OSX-KVM process: {}", error);
                self.osx_kvm_process = None;
            }
        }
    }

    // Import macos disk from osx kvm to reims-vgpu

    pub(crate) fn import_macos(&mut self) {
        if self.import_process.is_some() {
            return;
        }

        let home = match std::env::var_os("HOME") {
            Some(home) => std::path::PathBuf::from(home),
            None => {
                self.import_status = "Could not determine home directory.".to_string();
                return;
            }
        };

        let osx_kvm = download_root().join("OSX-KVM");

        let repo = if let Ok(path) = std::env::var("REIMS_VGPU_REPO") {
            std::path::PathBuf::from(path)
        } else {
            download_root().join("reims-vgpu")
        };

        let mac_hdd = osx_kvm.join("mac_hdd_ng.img");
        let opencore = osx_kvm.join("OpenCore").join("OpenCore.qcow2");
        let ovmf_code = osx_kvm.join("OVMF_CODE_4M.fd");
        let ovmf_vars = osx_kvm.join("OVMF_VARS-1920x1080.fd");

        if !repo.exists() {
            self.import_status = format!("reims-vGPU repository not found: {}", repo.display());
            return;
        }

        if !mac_hdd.exists() {
            self.import_status = format!("mac_hdd_ng.img not found: {}", mac_hdd.display());
            return;
        }

        if !opencore.exists() {
            self.import_status = format!("OpenCore not found: {}", opencore.display());
            return;
        }

        if !ovmf_code.exists() {
            self.import_status = format!("OVMF code not found: {}", ovmf_code.display());
            return;
        }

        if !ovmf_vars.exists() {
            self.import_status = format!("OVMF vars not found: {}", ovmf_vars.display());
            return;
        }

        let rail = match self.macOS_version.as_str() {
            "macOS 11 Big Sur" => "macos-11",
            "macOS 12 Monterey" => "macos-12",
            "macOS 13 Ventura" => "macos-13",
            "macOS 14 Sonoma" => "macos-14",
            "macOS 15 Sequoia" => "macos-15",
            "macOS 26 Tahoe" => "macos-26",
            _ => {
                self.import_status = "Unknown macOS version.".to_string();
                return;
            }
        };

        let base = repo
        .join("vm")
        .join("disks")
        .join("rails")
        .join(rail)
        .join("snapshots")
        .join("base");

        if let Err(error) = std::fs::create_dir_all(&base) {
            self.import_status = format!("Could not create snapshot directory: {}", error);
            return;
        }

        let destination = base.join("macos.img");

        self.import_status = "Copying macOS disk...".to_string();

        match Command::new("cp")
        .arg("--reflink=auto")
        .arg(&mac_hdd)
        .arg(&destination)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        {
            Ok(process) => {
                self.import_process = Some(process);
            }

            Err(error) => {
                self.import_status = format!("Could not start disk import: {}", error);
            }
        }
    }

    // update status on import

    pub(crate) fn update_import(&mut self, ctx: &egui::Context) {
        let Some(process) = &mut self.import_process else {
            return;
        };

        match process.try_wait() {
            Ok(Some(status)) => {
                self.import_process = None;

                if !status.success() {
                    self.import_status = "macOS disk import failed.".to_string();
                    return;
                }

                let home = match std::env::var_os("HOME") {
                    Some(home) => std::path::PathBuf::from(home),
                    None => {
                        self.import_status = "Could not determine home directory.".to_string();
                        return;
                    }
                };

                let osx_kvm = download_root().join("OSX-KVM");

                let repo = if let Ok(path) = std::env::var("REIMS_VGPU_REPO") {
                    std::path::PathBuf::from(path)
                } else {
                    download_root().join("reims-vgpu")
                };

                let rail = match self.macOS_version.as_str() {
                    "macOS 11 Big Sur" => "macos-11",
                    "macOS 12 Monterey" => "macos-12",
                    "macOS 13 Ventura" => "macos-13",
                    "macOS 14 Sonoma" => "macos-14",
                    "macOS 15 Sequoia" => "macos-15",
                    "macOS 26 Tahoe" => "macos-26",
                    _ => {
                        self.import_status = "Unknown macOS version.".to_string();
                        return;
                    }
                };

                let base = repo
                .join("vm")
                .join("disks")
                .join("rails")
                .join(rail)
                .join("snapshots")
                .join("base");

                let source_opencore = osx_kvm.join("OpenCore").join("OpenCore.qcow2");
                let source_ovmf_code = osx_kvm.join("OVMF_CODE_4M.fd");
                let source_ovmf_vars = osx_kvm.join("OVMF_VARS-1920x1080.fd");

                let destination_opencore = base.join("OpenCore.qcow2");
                let destination_ovmf_code = base.join("OVMF_CODE.fd");
                let destination_ovmf_vars = base.join("OVMF_VARS.fd");

                self.import_status = "Copying OpenCore and OVMF...".to_string();

                if !source_opencore.exists() {
                    self.import_status = format!(
                        "Source OpenCore does not exist: {}",
                        source_opencore.display()
                    );
                    return;
                }

                if !base.exists() {
                    self.import_status =
                    format!("Destination directory does not exist: {}", base.display());
                    return;
                }

                if let Err(error) = std::fs::copy(&source_opencore, &destination_opencore) {
                    self.import_status = format!(
                        "Could not copy OpenCore from {} to {}: {}",
                        source_opencore.display(),
                                                destination_opencore.display(),
                                                error
                    );
                    return;
                }

                if let Err(error) = std::fs::copy(&source_ovmf_code, &destination_ovmf_code) {
                    self.import_status = format!("Could not copy OVMF code: {}", error);
                    return;
                }

                if let Err(error) = std::fs::copy(&source_ovmf_vars, &destination_ovmf_vars) {
                    self.import_status = format!("Could not copy OVMF vars: {}", error);
                    return;
                }

                let disk = base.join("macos.img");

                for path in [
                    &disk,
                    &destination_opencore,
                    &destination_ovmf_code,
                    &destination_ovmf_vars,
                ] {
                    let mut permissions = match std::fs::metadata(path) {
                        Ok(metadata) => metadata.permissions(),
                        Err(error) => {
                            self.import_status = format!(
                                "Could not read permissions for {}: {}",
                                path.display(),
                                                        error
                            );
                            return;
                        }
                    };

                    permissions.set_readonly(true);

                    if let Err(error) = std::fs::set_permissions(path, permissions) {
                        self.import_status =
                        format!("Could not make {} read-only: {}", path.display(), error);
                        return;
                    }
                }

                let snapshots = base
                .parent()
                .expect("base directory should have snapshots parent");

                let current = snapshots.join("current");

                if current.exists() {
                    if let Err(error) = std::fs::remove_file(&current) {
                        self.import_status =
                        format!("Could not replace snapshots/current: {}", error);
                        return;
                    }
                }

                #[cfg(unix)]
                {
                    use std::os::unix::fs::symlink;

                    if let Err(error) = symlink("base", &current) {
                        self.import_status =
                        format!("Could not create snapshots/current: {}", error);
                        return;
                    }
                }

                self.import_status = "macOS successfully imported into reims-vGPU.".to_string();
            }

            Ok(None) => {
                ctx.request_repaint();
            }

            Err(error) => {
                self.import_status = format!("Failed to check import process: {}", error);
                self.import_process = None;
            }
        }
    }
}

