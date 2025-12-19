use std::env;
use std::path::PathBuf;

use magscope_file_combiner::{collect_errors, collect_warnings, combine_folder, GroupSummary};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: magscope_cli <folder>");
        std::process::exit(2);
    }

    let folder = PathBuf::from(&args[1]);
    if !folder.is_dir() {
        eprintln!("Error: not a folder: {}", folder.display());
        std::process::exit(2);
    }

    let report = combine_folder(&folder);
    print_report(&report);
}

fn print_report(report: &magscope_file_combiner::CombineReport) {
    println!("Folder: {}", report.folder.display());
    println!("Bead files: {}", report.bead_files);
    println!("Motor files: {}", report.motor_files);

    if report.bead_files == 0 && report.motor_files == 0 {
        println!("No matching files found.");
        return;
    }

    match report.bead.as_ref() {
        Some(summary) => print_group(summary, "Bead"),
        None => println!("Bead output: (not created)"),
    }

    match report.motor.as_ref() {
        Some(summary) => print_group(summary, "Motor"),
        None => println!("Motor output: (not created)"),
    }

    let warnings = collect_warnings(report);
    if !warnings.is_empty() {
        println!("Warnings:");
        for warning in warnings {
            println!("- {}: {}", warning.file.display(), warning.message);
        }
    }

    let errors = collect_errors(report);
    if !errors.is_empty() {
        println!("Errors:");
        for error in errors {
            if let Some(file) = error.file {
                println!("- {}: {}", file.display(), error.message);
            } else {
                println!("- {}", error.message);
            }
        }
    }
}

fn print_group(summary: &GroupSummary, label: &str) {
    let output = summary
        .output_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(not created)".to_string());

    println!(
        "{} output: {} (lines: {})",
        label, output, summary.data_lines
    );

}
