use eframe::egui;
use std::process::{Child, Command, Stdio};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
        .with_inner_size([700.0, 500.0])
        .with_min_inner_size([600.0, 450.0])
        .with_max_inner_size([1200.0, 900.0])
        .with_transparent(true),
        ..Default::default()
    };

    eframe::run_native(
        "reims-vGPU",
        options,
        Box::new(|_cc| Ok(Box::new(ReimsVgpuApp::default()))),
    )
}

// The different pages in our application.
#[derive(PartialEq)]
enum Page {
    Dashboard,
    Requirements,
    InstallMacOS,
    Build,
    VirtualMachine,
    Logs,
}

struct ReimsVgpuApp {
    current_page: Page,
    system_checked: bool,
    checks: Vec<SystemCheck>,
    macOS_version: String,

    download_status: String,
    download_process: Option<Child>,

    disk_size_gb: u64,
    disk_status: String,
    disk_created: bool,

    installer_status: String,
    installer_process: Option<Child>,

    repo_status: String,
    repo_process: Option<Child>,

    osx_kvm_status: String,
    osx_kvm_process: Option<Child>,

    import_status: String,
    import_process: Option<Child>,

    build_status: String,
    build_process: Option<Child>,

    vm_rail: String,
    vm_device: String,
    vm_status: String,
    vm_process: Option<Child>,

    logo_texture: Option<egui::TextureHandle>,
    wallpaper_texture: Option<egui::TextureHandle>,

    download_product: String,
    download_log_path: Option<std::path::PathBuf>,
}

struct SystemCheck {
    name: String,
    status: bool,
    details: String,
}

impl Default for ReimsVgpuApp {
    fn default() -> Self {
        Self {
            current_page: Page::Dashboard,
            system_checked: false,
            checks: Vec::new(),
            macOS_version: "macOS 13 Ventura".to_string(),

            download_status: "Ready".to_string(),
            download_process: None,

            disk_size_gb: 64,
            disk_status: "Not created".to_string(),
            disk_created: false,

            installer_status: "Ready".to_string(),
            installer_process: None,

            repo_status: "Not downloaded".to_string(),
            repo_process: None,

            osx_kvm_status: "Not downloaded".to_string(),
            osx_kvm_process: None,

            import_status: "Not imported".to_string(),
            import_process: None,

            build_status: "Not built".to_string(),
            build_process: None,

            vm_rail: "macos-13".to_string(),
            vm_device: "reims-vgpu-pci".to_string(),
            vm_status: "Ready".to_string(),
            vm_process: None,

            download_product: String::new(),
            download_log_path: None,

            logo_texture: None,
            wallpaper_texture: None,
        }
    }
}

impl eframe::App for ReimsVgpuApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.load_logo(ctx);
        self.load_wallpaper(ctx);

        let mut visuals = egui::Visuals::dark();

        visuals.override_text_color = Some(
            egui::Color32::from_rgb(245, 248, 255)
        );

        ctx.set_visuals(visuals);
        ctx.set_pixels_per_point(1.0);

        self.update_download(ctx);
        self.update_installer(ctx);
        self.update_repo(ctx);
        self.update_osx_kvm(ctx);
        self.update_import(ctx);
        self.update_build(ctx);
        self.update_vm(ctx);

        //draw wallpaper behind everything
        if let Some(texture) = &self.wallpaper_texture {
            let painter = ctx.layer_painter(egui::LayerId::background());
            let rect = ctx.screen_rect();

            painter.image(
                texture.id(),
                          rect,
                          egui::Rect::from_min_max(
                              egui::Pos2::new(0.0, 0.0),
                                                   egui::Pos2::new(1.0, 1.0),
                          ),
                          egui::Color32::WHITE,
            );
        }

        // Left sidebar
        egui::SidePanel::left("sidebar")
        .exact_width(235.0)
        .resizable(false)
        .frame(
            egui::Frame::none()
            .fill(egui::Color32::from_rgba_unmultiplied(
                8, 10, 18, 175,
            ))
            .stroke(egui::Stroke::new(
                1.0_f32,
                egui::Color32::from_rgba_unmultiplied(
                    255, 255, 255, 35,
                ),
            ))
            .inner_margin(16.0),
        )
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);

                if let Some(texture) = &self.logo_texture {
                    ui.image((texture.id(), egui::vec2(82.0, 82.0)));
                }

                ui.add_space(10.0);

                ui.heading(
                    egui::RichText::new("reims-vGPU")
                    .size(22.0)
                    .strong(),
                );

                ui.add_space(22.0);
            });

            ui.separator();
            ui.add_space(14.0);

            let nav_items = [
                (Page::Dashboard, "Dashboard"),
              (Page::Requirements, "Requirements"),
              (Page::InstallMacOS, "Install macOS"),
              (Page::Build, "Build"),
              (Page::VirtualMachine, "Virtual Machine"),
              (Page::Logs, "Logs"),
            ];

            for (page, label) in nav_items {
                let selected = self.current_page == page;

                if glass_button(ui, selected, label).clicked() {
                    self.current_page = page;
                }

                ui.add_space(6.0);
            }

            ui.with_layout(
                egui::Layout::bottom_up(egui::Align::LEFT),
                           |ui| {
                               ui.separator();
                               ui.add_space(10.0);
                               ui.label(
                                   egui::RichText::new("Linux")
                                   .size(13.0)
                                   .weak(),
                               );
                               ui.label(
                                   egui::RichText::new("x86_64")
                                   .size(13.0)
                                   .weak(),
                               );
                           },
            );
        });
        // Main content
        egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
            .fill(egui::Color32::from_rgba_unmultiplied(
                8, 10, 18, 35,
            ))
            .inner_margin(18.0),
        )
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                match self.current_page {
                    Page::Dashboard => self.show_dashboard(ui),
                  Page::Requirements => self.show_requirements(ui),
                  Page::InstallMacOS => self.show_install_macos(ui),
                  Page::Build => self.show_build(ui),
                  Page::VirtualMachine => self.show_vm(ui),
                  Page::Logs => self.show_logs(ui),
                }
            });
        });
    }
}

impl ReimsVgpuApp {
    fn show_dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading("Dashboard");

        ui.add_space(10.0);

        ui.label("reims-vGPU");
        ui.label("Run macOS with reims-vGPU on Linux.");

        ui.add_space(30.0);

        liquid_glass(ui, |ui| {
            ui.heading(
                egui::RichText::new("System Status")
                .size(22.0)
                .strong(),
            );

            ui.add_space(10.0);

            if self.system_checked {
                let passed = self.checks.iter().filter(|c| c.status).count();
                let total = self.checks.len();

                ui.label(
                    egui::RichText::new(
                        format!("{} / {} checks passed", passed, total),
                    )
                    .size(16.0)
                    .color(egui::Color32::from_rgb(245, 248, 255)),
                );
            } else {
                ui.label(
                    egui::RichText::new("System has not been checked yet.")
                    .size(16.0)
                    .color(egui::Color32::from_rgb(235, 240, 248)),
                );
            }

            ui.add_space(20.0);

            if glass_action_button(ui, "Check System").clicked() {
                self.check_system();
            }
        });

        ui.add_space(30.0);

        ui.heading("Quick Actions");

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if glass_action_button(ui, "Requirements").clicked() {
                self.current_page = Page::Requirements;
            }

            if glass_action_button(ui, "Build QEMU").clicked() {
                self.current_page = Page::Build;
            }

            if glass_action_button(ui, "Launch VM").clicked() {
                self.current_page = Page::VirtualMachine;
            }
        });
    }

    fn show_requirements(&mut self, ui: &mut egui::Ui) {
        ui.heading("System Requirements");

        ui.add_space(10.0);

        ui.label("Check whether your system is ready to run reims-vGPU.");

        ui.add_space(20.0);

        if glass_action_button(ui, "Check System").clicked() {
            self.check_system();
        }

        ui.add_space(20.0);

        if !self.system_checked {
            ui.label("No checks have been performed yet.");
        } else {
            for check in &self.checks {
                glass_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if check.status {
                            ui.label("✓");
                        } else {
                            ui.label("✗");
                        }

                        ui.label(&check.name);
                        ui.label(&check.details);
                    });
                });

                ui.add_space(5.0);
            }
        }

        ui.add_space(30.0);

        ui.heading("reims-vGPU Repository");

        ui.add_space(10.0);

        if self.repo_process.is_some() {
            ui.label("Downloading reims-vGPU...");
        } else {
            let repo_exists = download_root().join("reims-vgpu")
                .exists();

            if repo_exists {
                ui.label("reims-vGPU repository is installed.");
            } else if glass_action_button(ui, "Download reims-vGPU").clicked() {
                self.download_repo();
            }
        }

        ui.add_space(10.0);

        ui.label("Status:");

        if self.repo_process.is_some() {
            ui.label("Downloading...");
        } else {
            let repo_exists = download_root()
            .join("reims-vgpu")
            .exists();

            if repo_exists {
                ui.label("Installed");
            } else {
                ui.label(&self.repo_status);
            }
        }
    }

    fn show_install_macos(&mut self, ui: &mut egui::Ui) {
        ui.heading("Install macOS");

        ui.add_space(10.0);

        ui.label("Prepare a macOS virtual machine for reims-vGPU.");

        ui.add_space(25.0);

        glass_frame().show(ui, |ui| {
            ui.heading("macOS Version");

            ui.add_space(10.0);

            ui.radio_value(
                &mut self.macOS_version,
                "macOS 26 Tahoe".to_string(),
                "macOS 26 Tahoe",
            );

            ui.radio_value(
                &mut self.macOS_version,
                "macOS 15 Sequoia".to_string(),
                "macOS 15 Sequoia",
            );

            ui.radio_value(
                &mut self.macOS_version,
                "macOS 14 Sonoma".to_string(),
                "macOS 14 Sonoma",
            );

            ui.radio_value(
                &mut self.macOS_version,
                "macOS 13 Ventura".to_string(),
                "macOS 13 Ventura",
            );

            ui.radio_value(
                &mut self.macOS_version,
                "macOS 12 Monterey".to_string(),
                "macOS 12 Monterey",
            );

            ui.radio_value(
                &mut self.macOS_version,
                "macOS 11 Big Sur".to_string(),
                "macOS 11 Big Sur",
            );
        });

        ui.add_space(20.0);

        glass_frame().show(ui, |ui| {
            ui.heading("OSX-KVM");

            ui.add_space(10.0);

            ui.label("Download the OSX-KVM environment used to install macOS.");

            ui.add_space(15.0);

            let osx_kvm_exists = std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .map(|home| home.join("OSX-KVM").exists())
                .unwrap_or(false);

            if self.osx_kvm_process.is_some() {
                ui.label("Downloading OSX-KVM...");
            } else if osx_kvm_exists {
                ui.label("OSX-KVM is installed.");
            } else if glass_action_button(ui, "Download OSX-KVM").clicked() {
                self.download_osx_kvm();
            }

            ui.add_space(10.0);

            ui.label("Status:");

            if self.osx_kvm_process.is_some() {
                ui.label("Downloading...");
            } else if osx_kvm_exists {
                ui.label("Installed");
            } else {
                ui.label(&self.osx_kvm_status);
            }
        });

        ui.add_space(20.0);

        glass_frame().show(ui, |ui| {
            ui.heading("Virtual Disk");

            ui.add_space(10.0);

            ui.add(egui::Slider::new(&mut self.disk_size_gb, 32..=256).text("Disk size (GB)"));

            ui.label(format!("Selected size: {} GB", self.disk_size_gb));

            ui.add_space(10.0);

            if self.disk_created {
                ui.label(&self.disk_status);
            } else if glass_action_button(ui, "Create macOS Disk").clicked() {
                self.create_virtual_disk();
            }
        });

        ui.add_space(20.0);

        glass_frame().show(ui, |ui| {
            ui.heading("Installation");

            ui.add_space(10.0);

            ui.label("Download the recovery image and install macOS onto the virtual disk.");

            ui.add_space(15.0);

            let recovery_path = macos_download_path(&self.macOS_version);
            let recovery_exists = recovery_path.exists();

            if self.download_process.is_some() {
                ui.label("Downloading macOS...");
            } else if recovery_exists {
                ui.label(
                    format!(
                        "{} recovery image is already downloaded.",
                        self.macOS_version
                    )
                );

                if glass_action_button(ui, "Download Again").clicked() {
                    self.download_macos();
                }
            } else if glass_action_button(ui, "Download macOS Installer").clicked() {
                self.download_macos();
            }

            ui.add_space(10.0);

            ui.label("Download status:");
            ui.label(&self.download_status);

            if !self.download_product.is_empty() {
                ui.label(
                    format!(
                        "Apple recovery product: {}",
                        self.download_product
                    )
                );
            }

            ui.add_space(15.0);

            if self.installer_process.is_some() {
                ui.label("macOS installer is running...");
            } else if recovery_exists {
                if glass_action_button(ui, "Run macOS Installer").clicked() {
                    self.run_macos_installer();
                }
            } else {
                ui.label(
                    "Download the selected macOS recovery image before running the installer."
                );
            }

            ui.add_space(10.0);

            ui.label("Installer status:");
            ui.label(&self.installer_status);
        });

        ui.add_space(20.0);

        glass_frame().show(ui, |ui| {
            ui.heading("reims-vGPU");

            ui.add_space(10.0);

            ui.label("Import the installed macOS guest into reims-vGPU.");

            ui.add_space(15.0);

            if self.import_process.is_some() {
                ui.label("Importing macOS guest...");
            } else {
                let ready = std::env::var_os("HOME")
                    .map(std::path::PathBuf::from)
                    .map(|home| home.join("OSX-KVM").join("mac_hdd_ng.img").exists())
                    .unwrap_or(false);

                if ready {
                    if glass_action_button(ui, "Import macOS to reims-vGPU").clicked() {
                        self.import_macos();
                    }
                } else {
                    ui.add_enabled(false, egui::Button::new("Import macOS to reims-vGPU"));
                }
            }

            ui.add_space(10.0);

            ui.label("Status:");
            ui.label(&self.import_status);
        });

        ui.add_space(30.0);

        ui.label("Note: macOS 13 Ventura is recommended for bring-up.");
    }
    fn show_build(&mut self, ui: &mut egui::Ui) {
        ui.heading("Build");

        ui.add_space(10.0);

        ui.label("Build the reims-vGPU QEMU backend.");

        ui.add_space(30.0);

        glass_frame().show(ui, |ui| {
            ui.heading("QEMU");

            ui.add_space(10.0);

            ui.label("Target: x86_64");
            ui.label("Backend: Vulkan");

            ui.add_space(15.0);

            if self.build_process.is_some() {
                ui.label("Building QEMU...");
            } else if self.build_status == "Built successfully." {
                ui.label("QEMU is built.");
            } else if glass_action_button(ui, "Build QEMU").clicked() {
                self.build_qemu();
            }

            ui.add_space(10.0);

            ui.label("Status:");
            ui.label(&self.build_status);
        });
    }

    fn show_vm(&mut self, ui: &mut egui::Ui) {
        ui.heading("Virtual Machine");

        ui.add_space(10.0);

        ui.label("Configure and launch the macOS virtual machine.");

        ui.add_space(30.0);

        glass_frame().show(ui, |ui| {
            ui.heading("Configuration");

            ui.add_space(10.0);

            ui.label("Architecture: x86_64");

            ui.label("Backend: Vulkan");

            ui.add_space(15.0);

            ui.label("Installed rails:");

            let installed_rails = self.get_installed_rails();

            if installed_rails.is_empty() {
                ui.label("No macOS rails have been imported yet.");
            } else {
                let selected_exists = installed_rails
                .iter()
                .any(|(rail, _)| rail == &self.vm_rail);

                if !selected_exists {
                    self.vm_rail = installed_rails[0].0.clone();
                }

                for (rail, name) in &installed_rails {
                    ui.radio_value(
                        &mut self.vm_rail,
                        rail.clone(),
                                   name,
                    );
                }
            }

            ui.add_space(20.0);

            ui.label("Graphics device:");

            ui.radio_value(
                &mut self.vm_device,
                "reims-vgpu-pci".to_string(),
                           "reims-vGPU",
            );

            ui.radio_value(
                &mut self.vm_device,
                "vmware-svga".to_string(),
                           "VMware SVGA",
            );

            ui.add_space(20.0);

            ui.label(format!("Selected rail: {}", self.vm_rail));

            let device_name = match self.vm_device.as_str() {
                "reims-vgpu-pci" => "reims-vGPU",
                "vmware-svga" => "VMware SVGA",
                _ => "Unknown",
            };

            ui.label(format!("Selected device: {}", device_name));

            ui.add_space(20.0);

            if self.vm_process.is_some() {
                ui.label("VM is running.");

                if glass_action_button(ui, "Stop VM").clicked() {
                    self.stop_vm();
                }
            } else if glass_action_button(ui, "Launch VM").clicked() {
                self.launch_vm();
            }

            ui.add_space(10.0);

            ui.label("Status:");

            ui.label(&self.vm_status);
        });
    }

    fn show_logs(&mut self, ui: &mut egui::Ui) {
        ui.heading("Logs");

        ui.add_space(10.0);

        ui.label("Detailed logs are disabled.");
        ui.label("Process status is shown on each operation page.");
    }

    fn check_system(&mut self) {
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

        // Check a command exists
        fn check_command(command: &str) -> (bool, String) {
            match Command::new(command).arg("--version").output() {
                Ok(output) if output.status.success() => {
                    let version = String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .next()
                        .unwrap_or("")
                        .to_string();

                    (true, version)
                }

                _ => (false, "Not installed".to_string()),
            }
        }

        // Git
        let (git_ok, git_version) = check_command("git");

        self.checks.push(SystemCheck {
            name: "Git".to_string(),
            status: git_ok,
            details: git_version,
        });

        // Rust
        let (rust_ok, rust_version) = check_command("rustc");

        self.checks.push(SystemCheck {
            name: "Rust".to_string(),
            status: rust_ok,
            details: rust_version,
        });

        // Cargo
        let (cargo_ok, cargo_version) = check_command("cargo");

        self.checks.push(SystemCheck {
            name: "Cargo".to_string(),
            status: cargo_ok,
            details: cargo_version,
        });

        // Ninja
        let (ninja_ok, ninja_version) = check_command("ninja");

        self.checks.push(SystemCheck {
            name: "Ninja".to_string(),
            status: ninja_ok,
            details: ninja_version,
        });

        // Meson
        let (meson_ok, meson_version) = check_command("meson");

        self.checks.push(SystemCheck {
            name: "Meson".to_string(),
            status: meson_ok,
            details: meson_version,
        });

        // C compiler
        let (cc_ok, cc_version) = check_command("cc");

        self.checks.push(SystemCheck {
            name: "C Compiler".to_string(),
            status: cc_ok,
            details: cc_version,
        });

        self.system_checked = true;
    }

    fn download_macos(&mut self) {
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

    fn create_virtual_disk(&mut self) {
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

    fn update_installer(&mut self, ctx: &egui::Context) {
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

    fn run_macos_installer(&mut self) {
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

    fn import_macos(&mut self) {
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

    fn update_import(&mut self, ctx: &egui::Context) {
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

    fn update_download(&mut self, ctx: &egui::Context) {
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

    fn update_repo(&mut self, ctx: &egui::Context) {
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

    fn download_repo(&mut self) {
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

    fn download_osx_kvm(&mut self) {
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
    fn update_osx_kvm(&mut self, ctx: &egui::Context) {
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
    fn build_qemu(&mut self) {
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
    fn update_build(&mut self, ctx: &egui::Context) {
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
    fn launch_vm(&mut self) {
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
    fn update_vm(&mut self, ctx: &egui::Context) {
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


    fn stop_vm(&mut self) {
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


    fn load_logo(&mut self, ctx: &egui::Context) {
        if self.logo_texture.is_some() {
            return;
        }

        let bytes = include_bytes!("../assets/logo.png");

        let image = match image::load_from_memory(bytes) {
            Ok(image) => image.to_rgba8(),
            Err(error) => {
                eprintln!("Failed to load logo.png: {}", error);
                return;
            }
        };

        let size = [image.width() as usize, image.height() as usize];

        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            size,
            image.as_raw(),
        );

        self.logo_texture = Some(ctx.load_texture(
            "reims-vgpu-logo",
            color_image,
            egui::TextureOptions::LINEAR,
        ));
    }

    fn load_wallpaper(&mut self, ctx: &egui::Context) {
        if self.wallpaper_texture.is_some() {
            return;
        }

        let bytes = include_bytes!("../assets/wallpaper.png");

        let image = match image::load_from_memory(bytes) {
            Ok(image) => image.to_rgba8(),
            Err(error) => {
                eprintln!("Failed to load wallpaper.png: {}", error);
                return;
            }
        };

        let size = [image.width() as usize, image.height() as usize];

        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            size,
            image.as_raw(),
        );

        self.wallpaper_texture = Some(ctx.load_texture(
            "reims-vgpu-wallpaper",
            color_image,
            egui::TextureOptions::LINEAR,
        ));
    }

    fn get_installed_rails(&self) -> Vec<(String, String)> {
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
}

fn glass_frame() -> egui::Frame {
    egui::Frame::none()
    .fill(egui::Color32::from_rgba_unmultiplied(
        6, 8, 15, 145,
    ))
    .rounding(egui::Rounding::same(18.0))
    .inner_margin(22.0)
    .outer_margin(5.0)
}

fn liquid_glass<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let frame = egui::Frame::none()
    .fill(egui::Color32::from_rgba_unmultiplied(
        8, 10, 18, 220,
    ))
    .rounding(egui::Rounding::same(18.0))
    .inner_margin(22.0)
    .outer_margin(5.0);

    frame.show(ui, |ui| {
        add_contents(ui)
    })
}

fn glass_button(
    ui: &mut egui::Ui,
    selected: bool,
    text: &str,
) -> egui::Response {
    let fill = if selected {
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 48)
    } else {
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 22)
    };

    ui.add(
        egui::Button::new(
            egui::RichText::new(text)
            .color(egui::Color32::from_rgb(245, 248, 255))
            .size(16.0),
        )
        .fill(fill)
        .stroke(egui::Stroke::NONE),
    )
}

fn glass_action_button(
    ui: &mut egui::Ui,
    text: &str,
) -> egui::Response {
    ui.add(
        egui::Button::new(
            egui::RichText::new(text)
            .color(egui::Color32::from_rgb(245, 248, 255))
            .size(16.0),
        )
        .fill(egui::Color32::from_rgba_unmultiplied(
            255, 255, 255, 22,
        ))
        .stroke(egui::Stroke::NONE),
    )
}

fn download_root() -> std::path::PathBuf {
    std::env::var_os("HOME")
    .map(std::path::PathBuf::from)
    .unwrap_or_else(|| std::path::PathBuf::from("."))
    .join("reims-gui-download")
}

fn macos_download_path(version: &str) -> std::path::PathBuf {
    download_root()
    .join("macos")
    .join(version)
    .join("BaseSystem.dmg")
}
