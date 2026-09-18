use eframe::egui;

use crate::app::{Page, ReimsVgpuApp};

use crate::paths::{
    download_root,
    macos_download_path,
    osx_kvm_installed,
    reims_vgpu_installed,
};


// glass box frame

pub(crate) fn glass_frame() -> egui::Frame {
    egui::Frame::none()
    .fill(egui::Color32::from_rgba_unmultiplied(
        6, 8, 15, 145,
    ))
    .rounding(egui::Rounding::same(18.0))
    .inner_margin(22.0)
    .outer_margin(5.0)
}

// something for liquid glass

pub(crate) fn liquid_glass<R>(
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

// nice glass button

pub(crate) fn glass_button(
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

// nice glass button but action

pub(crate) fn glass_action_button(
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


// end of button and box stuff


impl ReimsVgpuApp {
    pub(crate) fn show_dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading(
            egui::RichText::new("Dashboard")
            .size(24.0),
        );


        ui.add_space(10.0);

        ui.label(
            egui::RichText::new("reims-vGPU")
            .size(15.0),
        );

        ui.label(
            egui::RichText::new("Run macOS with reims-vGPU on Linux.")
            .size(13.0),
        );

        ui.add_space(30.0);

        liquid_glass(ui, |ui| {
            ui.heading("System Status");

            ui.add_space(10.0);

            if self.system_checked {
                let passed = self.checks.iter().filter(|c| c.status).count();
                let total = self.checks.len();

                ui.label(
                    egui::RichText::new(
                        format!("{} / {} checks passed", passed, total),
                    )
                    .size(14.0)
                    .color(egui::Color32::from_rgb(245, 248, 255)),
                );
            } else {
                ui.label(
                    egui::RichText::new("System has not been checked yet.")
                    .size(14.0)
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

    // show system requirements window

    pub(crate) fn show_requirements(&mut self, ui: &mut egui::Ui) {
        ui.heading(
            egui::RichText::new("Requirements")
            .size(24.0),
        );

        ui.add_space(10.0);

        ui.label(
            egui::RichText::new("Check whether your system is ready to run reims-vGPU.")
            .size(15.0),
        );

        ui.add_space(20.0);


        ui.add_space(20.0);

        glass_frame().show(ui, |ui| {
            ui.heading("System Check");

            ui.add_space(10.0);

            if !self.system_checked {
                ui.label("No checks have been performed yet.");
                ui.add_space(20.0);
                if glass_action_button(ui, "Check System").clicked() {
                    self.check_system();
                }

            } else {
                for check in &self.checks {
                    ui.horizontal(|ui| {
                        if check.status {
                            ui.label("✓");
                        } else {
                            ui.label("✗");
                        }

                        ui.label(&check.name);
                        ui.label(&check.details);
                    });

                    ui.add_space(8.0);
                }
            }
        });

        ui.add_space(30.0);
        glass_frame().show(ui, |ui| {
            ui.heading("reims-vGPU Repository");

            ui.add_space(10.0);

            if self.repo_process.is_some() {
                ui.label("Downloading reims-vGPU...");
            } else {
                let repo_exists = reims_vgpu_installed();

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
                let repo_exists = reims_vgpu_installed();

                if repo_exists {
                    ui.label("Installed");
                } else {
                    ui.label(&self.repo_status);
                }
            }
        });
    }

    // show install macos window

    pub(crate) fn show_install_macos(&mut self, ui: &mut egui::Ui) {
        ui.heading(
            egui::RichText::new("Install macOS")
            .size(24.0),
        );

        ui.add_space(10.0);

        ui.label(
            egui::RichText::new("Prepare a macOS virtual machine for reims-vGPU.")
            .size(15.0),

        );

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

            let osx_kvm_exists = osx_kvm_installed();

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
            let recovery_dmg = recovery_path.join("BaseSystem.dmg");
            let recovery_exists = recovery_dmg.is_file();

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
                let ready = download_root()
                .join("OSX-KVM")
                .join("mac_hdd_ng.img")
                .exists();

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

    // show build window

    pub(crate) fn show_build(&mut self, ui: &mut egui::Ui) {
        ui.heading(
            egui::RichText::new("Build")
            .size(24.0),
        );

        ui.add_space(10.0);

        ui.label(
            egui::RichText::new("Build the reims-vGPU QEMU backend.")
            .size(15.0)
        );

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

    // show the vm window

    pub(crate) fn show_vm(&mut self, ui: &mut egui::Ui) {
        ui.heading(
            egui::RichText::new("Virtual Machine")
            .size(24.0),
        );

        ui.add_space(10.0);

        ui.label(
            egui::RichText::new("Configure and launch the macOS virtual machine.")
            .size(16.0),
        );

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

    // show logs lmao same name but different

    pub(crate) fn show_logs(&mut self, ui: &mut egui::Ui) {
        ui.heading(
            egui::RichText::new("Logs")
            .size(24.0),
        );

        ui.add_space(10.0);

        ui.label(
            egui::RichText::new("Detailed logs are disabled.")
            .size(15.0),
        );
        ui.label(
            egui::RichText::new("Process status is shown on each operation page.")
            .size(15.0),
        );
    }

    pub(crate) fn load_logo(&mut self, ctx: &egui::Context) {
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

    pub(crate) fn load_wallpaper(&mut self, ctx: &egui::Context) {
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
}



