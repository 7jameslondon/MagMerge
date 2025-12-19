use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

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

    let bin = env::var("CARGO_BIN_EXE_magscope_cli").expect("bin");
    let output = Command::new(bin)
        .arg(dir.path())
        .output()
        .expect("run cli");

    assert!(output.status.success());
    assert!(dir.path().join("Bead Positions Combined.txt").exists());
    assert!(dir.path().join("Motor Positions Combined.txt").exists());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Bead files: 1"));
    assert!(stdout.contains("Motor files: 1"));
}

#[test]
fn cli_reports_no_matching_files() {
    let dir = tempdir().expect("tempdir");
    let bin = env::var("CARGO_BIN_EXE_magscope_cli").expect("bin");
    let output = Command::new(bin)
        .arg(dir.path())
        .output()
        .expect("run cli");

    assert!(output.status.success());
    assert!(!dir.path().join("Bead Positions Combined.txt").exists());
    assert!(!dir.path().join("Motor Positions Combined.txt").exists());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No matching files found."));
}
