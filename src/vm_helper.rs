use eframe::egui;
use std::process::{Command, Stdio};

use crate::app::ReimsVgpuApp;
use crate::paths::download_root;

impl ReimsVgpuApp {
    pub(crate) fn launch_vm(&mut self) {
        if self.vm_process.is_some() {
            return;
        }

        let repo = if let Ok(path) = std::env::var("REIMS_VGPU_REPO") {
            std::path::PathBuf::from(path)
        } else {
            download_root().join("reims-vgpu")
        };

        let boot_script = repo.join("vm/boot-x86.sh");

        if !repo.exists() {
            self.vm_status = "reims-vGPU repository not found.".to_string();
            return;
        }

        if !boot_script.exists() {
            self.vm_status = "vm/boot-x86.sh not found.".to_string();
            return;
        }

        let rail_dir = repo
        .join("vm")
        .join("disks")
        .join("rails")
        .join(&self.vm_rail)
        .join("snapshots")
        .join("current");

        if !rail_dir.exists() {
            self.vm_status = format!(
                "Rail {} has not been imported yet.",
                self.vm_rail
            );
            return;
        }

        if self.vm_device == "reims-vgpu-pci" {
            let qemu = repo
            .join("vendor")
            .join("qemu")
            .join("build")
            .join("qemu-system-x86_64");

            if !qemu.exists() {
                self.vm_status =
                "QEMU is not built. Build QEMU first.".to_string();
                return;
            }
        }

        self.vm_status = format!(
            "Starting {} with {}...",
            self.vm_rail,
            match self.vm_device.as_str() {
                "reims-vgpu-pci" => "reims-vGPU",
                "vmware-svga" => "VMware SVGA",
                _ => "unknown device",
            }
        );

        match Command::new("bash")
        .arg("vm/boot-x86.sh")
        .arg("--interactive")
        .arg("--device")
        .arg(&self.vm_device)
        .arg("--rail")
        .arg(&self.vm_rail)
        .current_dir(&repo)
        .env("REIMS_VGPU_BACKEND", "vulkan")
        .spawn()
        {
            Ok(child) => {
                self.vm_process = Some(child);

                self.vm_status = format!(
                    "{} started with {}.",
                    self.vm_rail,
                    match self.vm_device.as_str() {
                        "reims-vgpu-pci" => "reims-vGPU",
                        "vmware-svga" => "VMware SVGA",
                        _ => "unknown device",
                    }
                );
            }

            Err(error) => {
                self.vm_status =
                format!("Failed to launch VM: {}", error);
            }
        }
    }

// update status on vm status

    pub(crate) fn update_vm(&mut self, ctx: &egui::Context) {
        let Some(mut child) = self.vm_process.take() else {
            return;
        };

        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    self.vm_status = "VM exited.".to_string();
                } else {
                    self.vm_status =
                    format!("VM exited with status {}.", status);
                }
            }

            Ok(None) => {
                self.vm_process = Some(child);
                ctx.request_repaint();
            }

            Err(err) => {
                self.vm_status =
                format!("Failed to check VM: {}", err);
            }
        }
    }

    // function to stop vm in gui

    pub(crate) fn stop_vm(&mut self) {
        if let Some(mut child) = self.vm_process.take() {
            match child.kill() {
                Ok(_) => {
                    self.vm_status = "VM stopped.".to_string();
                }

                Err(err) => {
                    self.vm_status =
                    format!("Failed to stop VM: {}", err);
                }
            }
        }
    }

    // get installed rail to show in vm launch

    pub(crate) fn get_installed_rails(&self) -> Vec<(String, String)> {
        let repo = if let Ok(path) = std::env::var("REIMS_VGPU_REPO") {
            std::path::PathBuf::from(path)
        } else {
            download_root().join("reims-vgpu")
        };

        let rails_dir = repo.join("vm/disks/rails");

        let rail_names = [
            ("macos-11", "macOS 11 Big Sur"),
            ("macos-12", "macOS 12 Monterey"),
            ("macos-13", "macOS 13 Ventura"),
            ("macos-14", "macOS 14 Sonoma"),
            ("macos-15", "macOS 15 Sequoia"),
            ("macos-26", "macOS 26 Tahoe"),
        ];

        rail_names
        .into_iter()
        .filter(|(rail, _)| {
            rails_dir
            .join(rail)
            .join("snapshots")
            .join("current")
            .exists()
        })
        .map(|(rail, name)| (rail.to_string(), name.to_string()))
        .collect()
    }

    pub(crate) fn update_repo(&mut self, ctx: &egui::Context) {
        let Some(process) = &mut self.repo_process else {
            return;
        };

        match process.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    self.repo_status = "Repository downloaded!".to_string();
                } else {
                    self.repo_status = "Repository download failed.".to_string();
                }

                self.repo_process = None;
            }

            Ok(None) => {
                // Git is still cloning.
                ctx.request_repaint();
            }

            Err(error) => {
                self.repo_status = format!("Failed to check git process: {}", error);

                self.repo_process = None;
            }
        }
    }

    pub(crate) fn download_repo(&mut self) {
        if self.repo_process.is_some() {
            return;
        }

        let repo_path = download_root().join("reims-vgpu");

        if repo_path.exists() {
            self.repo_status = format!("Repository already exists: {}", repo_path.display());
            return;
        }

        let parent = match repo_path.parent() {
            Some(path) => path,
            None => {
                self.repo_status = "Invalid repository path.".to_string();
                return;
            }
        };

        if let Err(error) = std::fs::create_dir_all(parent) {
            self.repo_status = format!("Could not create repository directory: {}", error);
            return;
        }

        self.repo_status = "Downloading reims-vGPU repository...".to_string();

        match Command::new("git")
        .arg("clone")
        .arg("https://github.com/steelbrain/reims-vgpu.git")
        .arg(&repo_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        {
            Ok(process) => {
                self.repo_process = Some(process);
            }

            Err(error) => {
                self.repo_status = format!("Could not start git: {}", error);
            }
        }
    }

    pub(crate) fn build_qemu(&mut self) {
        if self.build_process.is_some() {
            return;
        }

        let repo = download_root().join("reims-vgpu");

        let script = repo.join("scripts/qemu-build/qemu-build.sh");

        if !repo.exists() {
            self.build_status = format!("reims-vGPU repository not found: {}", repo.display());
            return;
        }

        if !script.exists() {
            self.build_status = format!("QEMU build script not found: {}", script.display());
            return;
        }

        self.build_status = "Building QEMU with the Vulkan backend...".to_string();

        match Command::new("bash")
        .arg(&script)
        .arg("--target")
        .arg("x86_64")
        .arg("--backend")
        .arg("vulkan")
        .current_dir(&repo)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        {
            Ok(process) => {
                self.build_process = Some(process);
            }

            Err(error) => {
                self.build_status = format!("Could not start QEMU build: {}", error);
            }
        }
    }

    pub(crate) fn update_build(&mut self, ctx: &egui::Context) {
        let Some(process) = &mut self.build_process else {
            return;
        };

        match process.try_wait() {
            Ok(Some(status)) => {
                self.build_process = None;

                if status.success() {
                    let repo =
                    download_root().join("reims-vgpu");

                    let qemu = repo.join("vendor/qemu/build/qemu-system-x86_64");

                    if qemu.exists() {
                        self.build_status = "Built successfully.".to_string();
                    } else {
                        self.build_status =
                        "Build finished but QEMU binary was not found.".to_string();
                    }
                } else {
                    self.build_status = "QEMU build failed.".to_string();
                }
            }

            Ok(None) => {
                ctx.request_repaint();
            }

            Err(error) => {
                self.build_status = format!("Failed to check QEMU build: {}", error);
                self.build_process = None;
            }
        }
    }
}

