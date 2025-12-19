use std::path::PathBuf;

use eframe::egui;
use magscope_file_combiner::{collect_errors, collect_warnings, combine_folder};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "MagScope File Combiner",
        options,
        Box::new(|_cc| Box::new(CombinerApp::default())),
    )
}

#[derive(Default)]
struct CombinerApp {
    folder: Option<PathBuf>,
    report: Option<magscope_file_combiner::CombineReport>,
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
                    self.report = Some(combine_folder(&folder));
                    break;
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("MagScope File Combiner");
            ui.label("Drop a folder here to combine Bead and Motor files.");

            if let Some(folder) = &self.folder {
                ui.label(format!("Folder: {}", folder.display()));
            }

            if let Some(report) = &self.report {
                ui.separator();
                ui.label(format!("Bead files: {}", report.bead_files));
                ui.label(format!("Motor files: {}", report.motor_files));

                if report.bead_files == 0 && report.motor_files == 0 {
                    ui.label("No matching files found.");
                }

                if let Some(summary) = report.bead.as_ref() {
                    let output = summary
                        .output_path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "(not created)".to_string());
                    ui.label(format!(
                        "Bead output: {} (lines: {})",
                        output, summary.data_lines
                    ));
                } else {
                    ui.label("Bead output: (not created)");
                }

                if let Some(summary) = report.motor.as_ref() {
                    let output = summary
                        .output_path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "(not created)".to_string());
                    ui.label(format!(
                        "Motor output: {} (lines: {})",
                        output, summary.data_lines
                    ));
                } else {
                    ui.label("Motor output: (not created)");
                }

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
    }
}
