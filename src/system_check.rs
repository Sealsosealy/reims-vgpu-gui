use std::process::Command;

use crate::app::{ReimsVgpuApp, SystemCheck};

impl ReimsVgpuApp {
    pub(crate) fn check_system(&mut self) {
            self.checks.clear();

            // Check Linux
            let is_linux = std::env::consts::OS == "linux";

            self.checks.push(SystemCheck {
                name: "Linux".to_string(),
                            status: is_linux,
                            details: if is_linux {
                                "Detected".to_string()
                            } else {
                                "Not Linux".to_string()
                            },
            });

            // Check x86_64
            let is_x86_64 = std::env::consts::ARCH == "x86_64";

            self.checks.push(SystemCheck {
                name: "x86_64".to_string(),
                            status: is_x86_64,
                            details: std::env::consts::ARCH.to_string(),
            });

            // Check KVM
            let kvm_exists = std::path::Path::new("/dev/kvm").exists();

            self.checks.push(SystemCheck {
                name: "KVM".to_string(),
                            status: kvm_exists,
                            details: if kvm_exists {
                                "/dev/kvm found".to_string()
                            } else {
                                "/dev/kvm not found".to_string()
                            },
            });

            // Check NVIDIA
            let nvidia = Command::new("nvidia-smi")
            .arg("--query-gpu=name")
            .arg("--format=csv,noheader")
            .output();

            match nvidia {
            Ok(output) if output.status.success() => {
            let gpu = String::from_utf8_lossy(&output.stdout).trim().to_string();

            self.checks.push(SystemCheck {
                name: "NVIDIA GPU".to_string(),
                            status: true,
                            details: gpu,
                });
            }

            _ => {
                self.checks.push(SystemCheck {
                    name: "NVIDIA GPU".to_string(),
                                status: false,
                                details: "nvidia-smi unavailable".to_string(),
                });
            }
        }
        self.system_checked = true;
    }
}
