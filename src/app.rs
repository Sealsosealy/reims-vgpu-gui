use eframe::egui;
use std::process::Child;

use crate::ui_helper::glass_button;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
    Dashboard,
    Requirements,
    InstallMacOS,
    Build,
    VirtualMachine,
    Logs,
    Updates,
}

pub(crate) struct ReimsVgpuApp {
    pub(crate) current_page: Page,
    pub(crate) system_checked: bool,
    pub(crate) checks: Vec<SystemCheck>,
    pub(crate) macOS_version: String,

    pub(crate) download_status: String,
    pub(crate) download_process: Option<Child>,

    pub(crate) disk_size_gb: u64,
    pub(crate) disk_status: String,
    pub(crate) disk_created: bool,

    pub(crate) installer_status: String,
    pub(crate) installer_process: Option<Child>,

    pub(crate) repo_status: String,
    pub(crate) repo_process: Option<Child>,

    pub(crate) osx_kvm_status: String,
    pub(crate) osx_kvm_process: Option<Child>,

    pub(crate) import_status: String,
    pub(crate) import_process: Option<Child>,

    pub(crate) build_status: String,
    pub(crate) build_process: Option<Child>,

    pub(crate) vm_rail: String,
    pub(crate) vm_device: String,
    pub(crate) vm_status: String,
    pub(crate) vm_process: Option<Child>,

    pub(crate) logo_texture: Option<egui::TextureHandle>,
    pub(crate) wallpaper_texture: Option<egui::TextureHandle>,

    pub(crate) download_product: String,
    pub(crate) download_log_path: Option<std::path::PathBuf>,

    pub(crate) dependency_checks: Vec<DependencyCheck>,
    pub(crate) dependencies_checked: bool,
    pub(crate) dependencies_status: String,
    pub(crate) dependency_install_process: Option<Child>,

    pub(crate) gui_latest_version: String,
    pub(crate) gui_update_status: String,
    pub(crate) gui_update_available: bool,
    pub(crate) gui_update_check_process: Option<Child>,
    pub(crate) gui_update_process: Option<Child>,

    pub(crate) reims_local_commit: String,
    pub(crate) reims_remote_commit: String,
    pub(crate) reims_update_status: String,
    pub(crate) reims_update_available: bool,
    pub(crate) reims_update_check_process: Option<Child>,
    pub(crate) reims_update_process: Option<Child>,
}

pub(crate) struct DependencyCheck {
    pub(crate) name: String,
    pub(crate) command: String,
    pub(crate) installed: bool,
    pub(crate) details: String,
}

pub(crate) struct SystemCheck {
    pub(crate) name: String,
    pub(crate) status: bool,
    pub(crate) details: String,
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

            dependency_checks: Vec::new(),
            dependencies_checked: false,
            dependencies_status: "Not checked".to_string(),
            dependency_install_process: None,

            gui_latest_version: String::new(),
            gui_update_status: "Not checked".to_string(),
            gui_update_available: false,
            gui_update_check_process: None,
            gui_update_process: None,

            reims_local_commit: String::new(),
            reims_remote_commit: String::new(),
            reims_update_status: "Not checked".to_string(),
            reims_update_available: false,
            reims_update_check_process: None,
            reims_update_process: None,
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
        self.update_dependency_install(ctx);
        self.update_gui_check(ctx);
        self.update_reims_check(ctx);
        self.update_gui_install(ctx);
        self.update_reims_install(ctx);

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
                    egui::RichText::new("reims-vGPU GUI")
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
