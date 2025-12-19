#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use eframe::egui;
use magmerge::{
    collect_errors, collect_warnings, combine_folder_with_progress, format_group_output,
    CombineReport, ProgressEvent,
};

fn main() -> eframe::Result<()> {
    let mut options = eframe::NativeOptions::default();
    if let Some(icon) = load_app_icon() {
        options.viewport = options.viewport.with_icon(Arc::new(icon));
    }

    eframe::run_native(
        "MagMerge",
        options,
        Box::new(|_cc| Box::new(CombinerApp::default())),
    )
}

fn load_app_icon() -> Option<egui::IconData> {
    let png_bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/logo.png"));
    match eframe::icon_data::from_png_bytes(png_bytes) {
        Ok(icon) => Some(icon),
        Err(_) => None,
    }
}

struct CombinerApp {
    folder: Option<PathBuf>,
    report: Option<CombineReport>,
    status_message: Option<String>,
    processing: bool,
    processed_files: usize,
    total_files: usize,
    current_file: Option<String>,
    discovered_bead: usize,
    discovered_motor: usize,
    scanning: bool,
    progress_rx: Option<mpsc::Receiver<ProgressEvent>>,
    result_rx: Option<mpsc::Receiver<CombineReport>>,
}

impl Default for CombinerApp {
    fn default() -> Self {
        Self {
            folder: None,
            report: None,
            status_message: None,
            processing: false,
            processed_files: 0,
            total_files: 0,
            current_file: None,
            discovered_bead: 0,
            discovered_motor: 0,
            scanning: false,
            progress_rx: None,
            result_rx: None,
        }
    }
}

impl eframe::App for CombinerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped_files.is_empty() {
            for dropped in dropped_files {
                if let Some(path) = dropped.path {
                    let folder = if path.is_dir() {
                        path
                    } else {
                        path.parent().unwrap_or(&path).to_path_buf()
                    };
                    self.folder = Some(folder.clone());
                    self.report = None;
                    self.status_message = Some(format!(
                        "Folder received. Scanning: {}",
                        folder.display()
                    ));
                    self.processing = true;
                    self.processed_files = 0;
                    self.total_files = 0;
                    self.current_file = None;
                    self.discovered_bead = 0;
                    self.discovered_motor = 0;
                    self.scanning = true;

                    let (progress_tx, progress_rx) = mpsc::channel();
                    let (result_tx, result_rx) = mpsc::channel();
                    self.progress_rx = Some(progress_rx);
                    self.result_rx = Some(result_rx);

                    thread::spawn(move || {
                        let report = combine_folder_with_progress(&folder, |update| {
                            let _ = progress_tx.send(update);
                        });
                        let _ = result_tx.send(report);
                    });
                    break;
                }
            }
        }

        if let Some(rx) = &self.progress_rx {
            while let Ok(update) = rx.try_recv() {
                match update {
                    ProgressEvent::Discovery {
                        bead_files,
                        motor_files,
                    } => {
                        self.discovered_bead = bead_files;
                        self.discovered_motor = motor_files;
                        self.status_message = Some(format!(
                            "Scanning files... found bead: {}, motor: {}",
                            bead_files, motor_files
                        ));
                        self.scanning = true;
                    }
                    ProgressEvent::Combine {
                        processed_files,
                        total_files,
                        file_type,
                        current_file,
                    } => {
                        self.scanning = false;
                        self.processed_files = processed_files;
                        self.total_files = total_files;
                        let file_name = current_file
                            .file_name()
                            .map(|name| name.to_string_lossy().to_string())
                            .unwrap_or_else(|| current_file.display().to_string());
                        let label = match file_type {
                            magmerge::FileType::Bead => "Bead",
                            magmerge::FileType::Motor => "Motor",
                        };
                        self.current_file = Some(format!("{label}: {file_name}"));
                        self.status_message = Some(format!(
                            "Processing file {}/{}",
                            self.processed_files, self.total_files
                        ));
                    }
                }
            }
        }

        if let Some(rx) = &self.result_rx {
            if let Ok(report) = rx.try_recv() {
                self.processing = false;
                self.progress_rx = None;
                self.result_rx = None;
                self.current_file = None;
                self.scanning = false;
                if report.bead_files == 0 && report.motor_files == 0 {
                    self.status_message = Some("No matching files found.".to_string());
                } else {
                    self.status_message = Some("Combine complete.".to_string());
                }
                self.report = Some(report);
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("MagMerge");
            ui.label("Drop a folder here to combine Bead and Motor files.");

            if let Some(folder) = &self.folder {
                ui.label(format!("Folder: {}", folder.display()));
            }

            if let Some(message) = &self.status_message {
                ui.label(message);
            }

            if self.scanning {
                ui.label(format!(
                    "Found so far: Bead {}, Motor {}",
                    self.discovered_bead, self.discovered_motor
                ));
            }

            if self.processing && self.total_files > 0 {
                let progress = self.processed_files as f32 / self.total_files as f32;
                ui.add(
                    egui::ProgressBar::new(progress).text(format!(
                        "{}/{}",
                        self.processed_files, self.total_files
                    )),
                );
                if let Some(current) = &self.current_file {
                    ui.label(format!("Processing: {}", current));
                }
            }

            if let Some(report) = &self.report {
                ui.separator();
                ui.label(format!("Bead files: {}", report.bead_files));
                ui.label(format!("Motor files: {}", report.motor_files));

                if report.bead_files == 0 && report.motor_files == 0 {
                    ui.label("No matching files found.");
                }

                ui.label(format_group_output(report.bead.as_ref(), "Bead"));
                ui.label(format_group_output(report.motor.as_ref(), "Motor"));

                let warnings = collect_warnings(report);
                let errors = collect_errors(report);

                if !warnings.is_empty() || !errors.is_empty() {
                    ui.separator();
                }

                if !warnings.is_empty() {
                    ui.label("Warnings:");
                    egui::ScrollArea::vertical()
                        .max_height(120.0)
                        .show(ui, |ui| {
                            for warning in warnings {
                                ui.label(format!(
                                    "- {}: {}",
                                    warning.file.display(),
                                    warning.message
                                ));
                            }
                        });
                }

                if !errors.is_empty() {
                    ui.label("Errors:");
                    egui::ScrollArea::vertical()
                        .max_height(120.0)
                        .show(ui, |ui| {
                            for error in errors {
                                if let Some(file) = error.file {
                                    ui.label(format!(
                                        "- {}: {}",
                                        file.display(),
                                        error.message
                                    ));
                                } else {
                                    ui.label(format!("- {}", error.message));
                                }
                            }
                        });
                }
            }
        });

        if self.processing {
            ctx.request_repaint();
        }
    }
}
