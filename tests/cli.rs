use std::fs;
use std::path::Path;

use tempfile::tempdir;

fn write_file(path: &Path, content: &str) {
    fs::write(path, content).expect("write file");
}

#[test]
fn cli_combines_files() {
    let dir = tempdir().expect("tempdir");
    write_file(
        &dir.path().join("Bead Positions 1.txt"),
        "# H\n1\n2\n",
    );
    write_file(
        &dir.path().join("Motor Positions 1.txt"),
        "# M\n3\n4\n",
    );

    let args = vec![
        "magscope_cli".to_string(),
        dir.path().to_string_lossy().to_string(),
    ];
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = magscope_file_combiner::cli::run_cli(&args, &mut stdout, &mut stderr);
    assert_eq!(exit_code, 0);
    assert!(dir.path().join("Bead Positions Combined.txt").exists());
    assert!(dir.path().join("Motor Positions Combined.txt").exists());

    let stdout = String::from_utf8_lossy(&stdout);
    assert!(stdout.contains("Bead files: 1"));
    assert!(stdout.contains("Motor files: 1"));
}

#[test]
fn cli_reports_no_matching_files() {
    let dir = tempdir().expect("tempdir");
    let args = vec![
        "magscope_cli".to_string(),
        dir.path().to_string_lossy().to_string(),
    ];
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = magscope_file_combiner::cli::run_cli(&args, &mut stdout, &mut stderr);
    assert_eq!(exit_code, 0);
    assert!(!dir.path().join("Bead Positions Combined.txt").exists());
    assert!(!dir.path().join("Motor Positions Combined.txt").exists());

    let stdout = String::from_utf8_lossy(&stdout);
    assert!(stdout.contains("No matching files found."));
}
