use eframe::egui;
use std::io::Read;
use std::process::{Command, Stdio};

use crate::app::{DependencyCheck, ReimsVgpuApp};
use crate::paths::reims_vgpu_path;

const GUI_REPO: &str = "https://github.com/Sealsosealy/reims-vgpu-gui.git";
const REIMS_VGPU_REPO: &str = "https://github.com/steelbrain/reims-vgpu.git";

fn parse_version(value: &str) -> Option<(u64, u64, u64)> {
    let clean = value.trim().trim_start_matches('v');
    let mut parts = clean.split('.');

    Some((
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ))
}

fn latest_version_from_tags(output: &str) -> Option<String> {
    output
        .lines()
        .filter_map(|line| {
            let tag = line.rsplit_once('\t')?.1.strip_prefix("refs/tags/")?;
            parse_version(tag).map(|version| (version, tag.to_string()))
        })
        .max_by_key(|(version, _)| *version)
        .map(|(_, tag)| tag)
}

fn read_child_stdout(child: &mut std::process::Child) -> Result<String, String> {
    let mut output = String::new();

    if let Some(mut stdout) = child.stdout.take() {
        stdout
            .read_to_string(&mut output)
            .map_err(|error| format!("Could not read process output: {}", error))?;
    }

    Ok(output)
}

fn command_exists(command: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg("command -v \"$1\" >/dev/null 2>&1")
        .arg("reims-vgpu-gui")
        .arg(command)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn distro_id() -> Option<String> {
    let contents = std::fs::read_to_string("/etc/os-release").ok()?;

    contents.lines().find_map(|line| {
        let value = line.strip_prefix("ID=")?;
        Some(value.trim_matches('"').to_ascii_lowercase())
    })
}

fn package_for(distro: &str, command: &str) -> Option<Vec<&'static str>> {
    match distro {
        "arch" | "manjaro" => match command {
            "git" => Some(vec!["git"]),
            "cargo" => Some(vec!["rust"]),
            "qemu-system-x86_64" => Some(vec!["qemu-desktop"]),
            "make" => Some(vec!["base-devel"]),
            "ninja" => Some(vec!["ninja"]),
            "python3" => Some(vec!["python"]),
            "dmg2img" => Some(vec!["dmg2img"]),
            "curl" => Some(vec!["curl"]),
            "pkg-config" => Some(vec!["pkgconf"]),
            "meson" => Some(vec!["meson"]),
            "gcc" => Some(vec!["base-devel"]),
            _ => None,
        },

        "debian" | "ubuntu" | "linuxmint" | "pop" => match command {
            "git" => Some(vec!["git"]),
            "cargo" => Some(vec!["cargo", "rustc"]),
            "qemu-system-x86_64" => Some(vec!["qemu-system-x86"]),
            "make" => Some(vec!["build-essential"]),
            "ninja" => Some(vec!["ninja-build"]),
            "python3" => Some(vec!["python3"]),
            "dmg2img" => Some(vec!["dmg2img"]),
            "curl" => Some(vec!["curl"]),
            "pkg-config" => Some(vec!["pkg-config"]),
            "meson" => Some(vec!["meson"]),
            "gcc" => Some(vec!["build-essential"]),
            _ => None,
        },

        "fedora" => match command {
            "git" => Some(vec!["git"]),
            "cargo" => Some(vec!["rust", "cargo"]),
            "qemu-system-x86_64" => Some(vec!["qemu-system-x86-core"]),
            "make" => Some(vec!["make"]),
            "ninja" => Some(vec!["ninja-build"]),
            "python3" => Some(vec!["python3"]),
            "dmg2img" => Some(vec!["dmg2img"]),
            "curl" => Some(vec!["curl"]),
            "pkg-config" => Some(vec!["pkgconf-pkg-config"]),
            "meson" => Some(vec!["meson"]),
            "gcc" => Some(vec!["gcc", "gcc-c++"]),
            _ => None,
        },

        _ => None,
    }
}

impl ReimsVgpuApp {
    pub(crate) fn check_dependencies(&mut self) {
        let specs = [
            ("Git", "git"),
            ("Rust / Cargo", "cargo"),
            ("QEMU", "qemu-system-x86_64"),
            ("Make", "make"),
            ("Ninja", "ninja"),
            ("Python 3", "python3"),
            ("dmg2img", "dmg2img"),
            ("curl", "curl"),
            ("pkg-config", "pkg-config"),
            ("Meson", "meson"),
            ("C compiler", "gcc"),
        ];

        self.dependency_checks.clear();

        for (name, command) in specs {
            let installed = command_exists(command);

            self.dependency_checks.push(DependencyCheck {
                name: name.to_string(),
                command: command.to_string(),
                installed,
                details: if installed {
                    "Installed".to_string()
                } else {
                    "Missing".to_string()
                },
            });
        }

        self.dependencies_checked = true;

        let missing = self
            .dependency_checks
            .iter()
            .filter(|dependency| !dependency.installed)
            .count();

        if missing == 0 {
            self.dependencies_status = "All required tools are installed.".to_string();
        } else {
            self.dependencies_status =
                format!("{} required tools are missing.", missing);
        }
    }

    pub(crate) fn install_missing_dependencies(&mut self) {
        if self.dependency_install_process.is_some() {
            return;
        }

        if !self.dependencies_checked {
            self.check_dependencies();
        }

        let missing_commands: Vec<String> = self
            .dependency_checks
            .iter()
            .filter(|dependency| !dependency.installed)
            .map(|dependency| dependency.command.clone())
            .collect();

        if missing_commands.is_empty() {
            self.dependencies_status = "All required tools are already installed.".to_string();
            return;
        }

        let distro = match distro_id() {
            Some(distro) => distro,
            None => {
                self.dependencies_status =
                    "Could not detect the Linux distribution.".to_string();
                return;
            }
        };

        let mut packages = Vec::<String>::new();

        for command in &missing_commands {
            let Some(package_names) = package_for(&distro, command) else {
                self.dependencies_status = format!(
                    "No automatic package mapping is available for {} on {}.",
                    command, distro
                );
                return;
            };

            for package in package_names {
                if !packages.iter().any(|existing| existing == package) {
                    packages.push(package.to_string());
                }
            }
        }

        self.dependencies_status = format!(
            "Installing {} package{}...",
            packages.len(),
            if packages.len() == 1 { "" } else { "s" }
        );

        let (program, prefix_args): (&str, Vec<&str>) = if command_exists("pkexec") {
            ("pkexec", Vec::new())
        } else if command_exists("sudo") {
            ("sudo", vec!["-n"])
        } else {
            self.dependencies_status =
                "Neither pkexec nor sudo is available. Install the missing tools manually."
                    .to_string();
            return;
        };

        let mut command = Command::new(program);
        command.args(&prefix_args);

        match distro.as_str() {
            "arch" | "manjaro" => {
                command.arg("pacman").args(["-S", "--needed"]);
            }

            "debian" | "ubuntu" | "linuxmint" | "pop" => {
                command.arg("apt-get").args(["install", "-y"]);
            }

            "fedora" => {
                command.arg("dnf").args(["install", "-y"]);
            }

            _ => {
                self.dependencies_status =
                    format!("Automatic installation is not supported on {} yet.", distro);
                return;
            }
        }

        command
            .args(&packages)
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        match command.spawn() {
            Ok(process) => {
                self.dependency_install_process = Some(process);
            }

            Err(error) => {
                self.dependencies_status =
                    format!("Could not start dependency installer: {}", error);
            }
        }
    }

    pub(crate) fn update_dependency_install(&mut self, ctx: &egui::Context) {
        let Some(process) = &mut self.dependency_install_process else {
            return;
        };

        match process.try_wait() {
            Ok(Some(status)) => {
                self.dependency_install_process = None;

                if status.success() {
                    self.dependencies_status =
                        "Dependencies installed. Checking again...".to_string();
                    self.check_dependencies();
                } else {
                    self.dependencies_status =
                        "Dependency installation failed. Check your package manager."
                            .to_string();
                }
            }

            Ok(None) => {
                ctx.request_repaint();
            }

            Err(error) => {
                self.dependencies_status =
                    format!("Failed to check dependency installer: {}", error);
                self.dependency_install_process = None;
            }
        }
    }

    pub(crate) fn check_updates(&mut self) {
        if self.gui_update_check_process.is_some() || self.reims_update_check_process.is_some() {
            return;
        }

        self.gui_update_status = "Checking for GUI updates...".to_string();
        self.reims_update_status = "Checking reims-vGPU...".to_string();
        self.gui_update_available = false;
        self.reims_update_available = false;

        match Command::new("git")
            .args(["ls-remote", "--tags", "--refs", GUI_REPO])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(process) => {
                self.gui_update_check_process = Some(process);
            }

            Err(error) => {
                self.gui_update_status =
                    format!("Could not check GUI updates: {}", error);
            }
        }

        let repo = reims_vgpu_path();

        if !repo.join(".git").exists() {
            self.reims_update_status = "reims-vGPU repository is not a Git checkout.".to_string();
            return;
        }

        match Command::new("git")
            .args(["-C"])
            .arg(&repo)
            .args(["rev-parse", "HEAD"])
            .output()
        {
            Ok(output) if output.status.success() => {
                self.reims_local_commit =
                    String::from_utf8_lossy(&output.stdout).trim().to_string();
            }

            Ok(output) => {
                self.reims_update_status = format!(
                    "Could not read local reims-vGPU commit: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                );
            }

            Err(error) => {
                self.reims_update_status =
                    format!("Could not read local reims-vGPU commit: {}", error);
            }
        }

        match Command::new("git")
            .args(["ls-remote", REIMS_VGPU_REPO, "refs/heads/master"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(process) => {
                self.reims_update_check_process = Some(process);
            }

            Err(error) => {
                self.reims_update_status =
                    format!("Could not check reims-vGPU updates: {}", error);
            }
        }
    }

    pub(crate) fn update_gui_check(&mut self, ctx: &egui::Context) {
        let Some(process) = &mut self.gui_update_check_process else {
            return;
        };

        match process.try_wait() {
            Ok(Some(status)) => {
                let output = read_child_stdout(process).unwrap_or_default();
                self.gui_update_check_process = None;

                if !status.success() {
                    self.gui_update_status =
                        "Could not reach the GUI update source.".to_string();
                    return;
                }

                let Some(latest) = latest_version_from_tags(&output) else {
                    self.gui_update_status =
                        "No stable GUI release tags were found.".to_string();
                    return;
                };

                self.gui_latest_version = latest.clone();

                let current = env!("CARGO_PKG_VERSION");

                match (parse_version(current), parse_version(&latest)) {
                    (Some(current), Some(latest_version)) if latest_version > current => {
                        self.gui_update_available = true;
                        self.gui_update_status =
                            format!("Update available: {}", latest);
                    }

                    (Some(_), Some(_)) => {
                        self.gui_update_status =
                            format!("GUI is up to date ({})", current);
                    }

                    _ => {
                        self.gui_update_status =
                            format!("Latest GUI release: {}", latest);
                    }
                }
            }

            Ok(None) => {
                ctx.request_repaint();
            }

            Err(error) => {
                self.gui_update_status =
                    format!("Failed to check GUI update: {}", error);
                self.gui_update_check_process = None;
            }
        }
    }

    pub(crate) fn update_reims_check(&mut self, ctx: &egui::Context) {
        let Some(process) = &mut self.reims_update_check_process else {
            return;
        };

        match process.try_wait() {
            Ok(Some(status)) => {
                let output = read_child_stdout(process).unwrap_or_default();
                self.reims_update_check_process = None;

                if !status.success() {
                    self.reims_update_status =
                        "Could not reach the reims-vGPU repository.".to_string();
                    return;
                }

                let Some(remote) = output.split_whitespace().next() else {
                    self.reims_update_status =
                        "The upstream reims-vGPU commit could not be read.".to_string();
                    return;
                };

                self.reims_remote_commit = remote.to_string();

                if self.reims_local_commit == remote {
                    self.reims_update_status =
                        "reims-vGPU is up to date.".to_string();
                    self.reims_update_available = false;
                } else {
                    self.reims_update_status =
                        "A newer reims-vGPU commit is available.".to_string();
                    self.reims_update_available = true;
                }
            }

            Ok(None) => {
                ctx.request_repaint();
            }

            Err(error) => {
                self.reims_update_status =
                    format!("Failed to check reims-vGPU update: {}", error);
                self.reims_update_check_process = None;
            }
        }
    }

    pub(crate) fn install_gui_update(&mut self) {
        if self.gui_update_process.is_some() {
            return;
        }

        if cfg!(debug_assertions) {
            self.gui_update_status =
                "Self-update is disabled in debug builds. Build a release binary first."
                    .to_string();
            return;
        }

        if !self.gui_update_available {
            self.gui_update_status = "No GUI update is available.".to_string();
            return;
        }

        let current_exe = match std::env::current_exe() {
            Ok(path) => path,
            Err(error) => {
                self.gui_update_status =
                    format!("Could not find the running GUI: {}", error);
                return;
            }
        };

        let latest = self.gui_latest_version.clone();

        let download_url = format!(
            "https://github.com/Sealsosealy/reims-vgpu-gui/releases/download/{}/reims-vgpu-gui",
            latest
        );

        let temp_path = current_exe.with_extension("update");

        let script =
            "set -e; curl -fL --retry 3 \"$1\" -o \"$2\"; chmod +x \"$2\"; mv -f \"$2\" \"$3\"";

        match Command::new("sh")
            .arg("-c")
            .arg(script)
            .arg("reims-vgpu-gui-updater")
            .arg(download_url)
            .arg(&temp_path)
            .arg(&current_exe)
            .spawn()
        {
            Ok(process) => {
                self.gui_update_process = Some(process);
                self.gui_update_status =
                    format!("Downloading GUI {}...", latest);
            }

            Err(error) => {
                self.gui_update_status =
                    format!("Could not start GUI updater: {}", error);
            }
        }
    }

    pub(crate) fn update_gui_install(&mut self, ctx: &egui::Context) {
        let Some(process) = &mut self.gui_update_process else {
            return;
        };

        match process.try_wait() {
            Ok(Some(status)) => {
                self.gui_update_process = None;

                if status.success() {
                    self.gui_update_status =
                        "Update installed. Restart the GUI to use the new version.".to_string();
                    self.gui_update_available = false;
                } else {
                    self.gui_update_status =
                        "GUI update failed. The current version was left in place.".to_string();
                }
            }

            Ok(None) => {
                ctx.request_repaint();
            }

            Err(error) => {
                self.gui_update_status =
                    format!("Failed to check GUI updater: {}", error);
                self.gui_update_process = None;
            }
        }
    }

    pub(crate) fn update_reims_vgpu(&mut self) {
        if self.reims_update_process.is_some() {
            return;
        }

        if !self.reims_update_available {
            self.reims_update_status =
                "No reims-vGPU update is available.".to_string();
            return;
        }

        let repo = reims_vgpu_path();

        self.reims_update_status =
            "Updating reims-vGPU with a fast-forward-only pull...".to_string();

        match Command::new("git")
            .args(["-C"])
            .arg(&repo)
            .args([
                "pull",
                "--ff-only",
                REIMS_VGPU_REPO,
                "master",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(process) => {
                self.reims_update_process = Some(process);
            }

            Err(error) => {
                self.reims_update_status =
                    format!("Could not start reims-vGPU update: {}", error);
            }
        }
    }

    pub(crate) fn update_reims_install(&mut self, ctx: &egui::Context) {
        let Some(process) = &mut self.reims_update_process else {
            return;
        };

        match process.try_wait() {
            Ok(Some(status)) => {
                self.reims_update_process = None;

                if status.success() {
                    self.reims_update_status =
                        "reims-vGPU updated successfully. Run the check again to verify the commit."
                            .to_string();
                    self.reims_update_available = false;
                } else {
                    self.reims_update_status =
                        "reims-vGPU update failed. Local changes may prevent a fast-forward."
                            .to_string();
                }
            }

            Ok(None) => {
                ctx.request_repaint();
            }

            Err(error) => {
                self.reims_update_status =
                    format!("Failed to check reims-vGPU updater: {}", error);
                self.reims_update_process = None;
            }
        }
    }

    pub(crate) fn show_updates(&mut self, ui: &mut egui::Ui) {
        ui.heading(
            egui::RichText::new("Updates")
                .size(24.0),
        );

        ui.add_space(10.0);

        ui.label(
            egui::RichText::new(
                "Keep the GUI and the installed reims-vGPU checkout up to date.",
            )
            .size(15.0),
        );

        ui.add_space(25.0);

        crate::ui_helper::glass_frame().show(ui, |ui| {
            ui.heading("reims-vGPU GUI");

            ui.add_space(10.0);

            ui.label(format!("Current version: {}", env!("CARGO_PKG_VERSION")));

            if !self.gui_latest_version.is_empty() {
                ui.label(format!(
                    "Latest release: {}",
                    self.gui_latest_version
                ));
            }

            ui.add_space(10.0);

            if self.gui_update_check_process.is_some() || self.gui_update_process.is_some() {
                ui.label(&self.gui_update_status);
            } else {
                ui.label(&self.gui_update_status);
            }

            ui.add_space(10.0);

            if self.gui_update_process.is_some() {
                ui.label("GUI update is running...");
            } else if self.gui_update_available {
                if cfg!(debug_assertions) {
                    ui.label("Build the GUI in release mode to enable self-update.");
                } else if crate::ui_helper::glass_action_button(ui, "Update GUI").clicked() {
                    self.install_gui_update();
                }
            }

            if crate::ui_helper::glass_action_button(ui, "Check for Updates").clicked() {
                self.check_updates();
            }
        });

        ui.add_space(20.0);

        crate::ui_helper::glass_frame().show(ui, |ui| {
            ui.heading("reims-vGPU");

            ui.add_space(10.0);

            if self.reims_local_commit.is_empty() {
                ui.label("Installed commit: Not checked");
            } else {
                ui.label(format!(
                    "Installed commit: {}",
                    &self.reims_local_commit[..self.reims_local_commit.len().min(12)]
                ));
            }

            if self.reims_remote_commit.is_empty() {
                ui.label("Upstream commit: Not checked");
            } else {
                ui.label(format!(
                    "Upstream commit: {}",
                    &self.reims_remote_commit[..self.reims_remote_commit.len().min(12)]
                ));
            }

            ui.add_space(10.0);

            ui.label(&self.reims_update_status);

            ui.add_space(10.0);

            if self.reims_update_process.is_some() {
                ui.label("Updating reims-vGPU...");
            } else if self.reims_update_available {
                if crate::ui_helper::glass_action_button(ui, "Update reims-vGPU").clicked() {
                    self.update_reims_vgpu();
                }
            }
        });
    }
}
