use std::io::Write;
use std::path::PathBuf;

use crate::{collect_errors, collect_warnings, combine_folder, CombineReport, GroupSummary};

pub fn run_cli(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    if args.len() != 2 {
        let _ = writeln!(err, "Usage: magmerge_cli <folder>");
        return 2;
    }

    let folder = PathBuf::from(&args[1]);
    if !folder.is_dir() {
        let _ = writeln!(err, "Error: not a folder: {}", folder.display());
        return 2;
    }

    let report = combine_folder(&folder);
    print_report(&report, out);
    0
}

fn print_report(report: &CombineReport, out: &mut dyn Write) {
    let _ = writeln!(out, "Folder: {}", report.folder.display());
    let _ = writeln!(out, "Bead files: {}", report.bead_files);
    let _ = writeln!(out, "Motor files: {}", report.motor_files);

    if report.bead_files == 0 && report.motor_files == 0 {
        let _ = writeln!(out, "No matching files found.");
        return;
    }

    if let Some(summary) = report.bead.as_ref() {
        print_group(summary, "Bead", out);
    } else {
        let _ = writeln!(out, "Bead output: (not created)");
    }

    if let Some(summary) = report.motor.as_ref() {
        print_group(summary, "Motor", out);
    } else {
        let _ = writeln!(out, "Motor output: (not created)");
    }

    let warnings = collect_warnings(report);
    if !warnings.is_empty() {
        let _ = writeln!(out, "Warnings:");
        for warning in warnings {
            let _ = writeln!(out, "- {}: {}", warning.file.display(), warning.message);
        }
    }

    let errors = collect_errors(report);
    if !errors.is_empty() {
        let _ = writeln!(out, "Errors:");
        for error in errors {
            if let Some(file) = error.file {
                let _ = writeln!(out, "- {}: {}", file.display(), error.message);
            } else {
                let _ = writeln!(out, "- {}", error.message);
            }
        }
    }
}

fn print_group(summary: &GroupSummary, label: &str, out: &mut dyn Write) {
    let output = summary
        .output_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(not created)".to_string());

    let _ = writeln!(
        out,
        "{} output: {} (lines: {})",
        label, output, summary.data_lines
    );
}
