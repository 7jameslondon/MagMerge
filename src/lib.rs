use std::fs;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

pub mod cli;

#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub processed_files: usize,
    pub total_files: usize,
    pub file_type: FileType,
    pub current_file: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Bead,
    Motor,
}

#[derive(Debug, Clone)]
pub struct DiscoveredFiles {
    pub bead_files: Vec<PathBuf>,
    pub motor_files: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct Warning {
    pub file: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct Error {
    pub file: Option<PathBuf>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct GroupSummary {
    pub file_type: FileType,
    pub input_files: usize,
    pub output_path: Option<PathBuf>,
    pub data_lines: usize,
    pub header: Option<Vec<u8>>,
    pub warnings: Vec<Warning>,
    pub errors: Vec<Error>,
}

#[derive(Debug, Clone)]
pub struct CombineReport {
    pub folder: PathBuf,
    pub bead_files: usize,
    pub motor_files: usize,
    pub bead: Option<GroupSummary>,
    pub motor: Option<GroupSummary>,
    pub errors: Vec<Error>,
}

pub fn discover_files(folder: &Path) -> io::Result<DiscoveredFiles> {
    let mut bead_files = Vec::new();
    let mut motor_files = Vec::new();

    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "txt" {
            continue;
        }

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        if is_combined_output(&name) {
            continue;
        }

        match classify_name(&name) {
            Some(FileType::Bead) => bead_files.push(path),
            Some(FileType::Motor) => motor_files.push(path),
            None => {}
        }
    }

    sort_paths(&mut bead_files);
    sort_paths(&mut motor_files);

    Ok(DiscoveredFiles {
        bead_files,
        motor_files,
    })
}

pub fn combine_folder(folder: &Path) -> CombineReport {
    combine_folder_with_progress(folder, |_| {})
}

pub fn combine_folder_with_progress<F>(folder: &Path, mut on_progress: F) -> CombineReport
where
    F: FnMut(ProgressUpdate),
{
    let mut report = CombineReport {
        folder: folder.to_path_buf(),
        bead_files: 0,
        motor_files: 0,
        bead: None,
        motor: None,
        errors: Vec::new(),
    };

    let discovered = match discover_files(folder) {
        Ok(files) => files,
        Err(err) => {
            report.errors.push(Error {
                file: None,
                message: format!("Failed to scan folder: {err}"),
            });
            return report;
        }
    };

    report.bead_files = discovered.bead_files.len();
    report.motor_files = discovered.motor_files.len();
    let total_files = report.bead_files + report.motor_files;
    let mut processed_files = 0usize;
    let mut on_file_processed = |file_type: FileType, path: &PathBuf| {
        processed_files += 1;
        on_progress(ProgressUpdate {
            processed_files,
            total_files,
            file_type,
            current_file: path.clone(),
        });
    };

    if !discovered.bead_files.is_empty() {
        let output = folder.join(output_filename(FileType::Bead));
        report.bead = Some(combine_group_with_progress(
            FileType::Bead,
            &discovered.bead_files,
            &output,
            &mut on_file_processed,
        ));
    }

    if !discovered.motor_files.is_empty() {
        let output = folder.join(output_filename(FileType::Motor));
        report.motor = Some(combine_group_with_progress(
            FileType::Motor,
            &discovered.motor_files,
            &output,
            &mut on_file_processed,
        ));
    }

    report
}

pub fn combine_group(file_type: FileType, files: &[PathBuf], output_path: &Path) -> GroupSummary {
    combine_group_with_progress(file_type, files, output_path, &mut |_, _| {})
}

pub fn combine_group_with_progress<F>(
    file_type: FileType,
    files: &[PathBuf],
    output_path: &Path,
    on_file_processed: &mut F,
) -> GroupSummary
where
    F: FnMut(FileType, &PathBuf),
{
    let mut summary = GroupSummary {
        file_type,
        input_files: files.len(),
        output_path: None,
        data_lines: 0,
        header: None,
        warnings: Vec::new(),
        errors: Vec::new(),
    };

    if files.is_empty() {
        return summary;
    }

    let mut writer: Option<BufWriter<File>> = None;
    let mut header_ref: Option<Vec<u8>> = None;

    for path in files {
        let content = match read_file_content(path) {
            Ok(content) => content,
            Err(err) => {
                summary.errors.push(Error {
                    file: Some(path.clone()),
                    message: format!("Failed to read file: {err}"),
                });
                on_file_processed(file_type, path);
                continue;
            }
        };

        if writer.is_none() {
            match File::create(output_path) {
                Ok(file) => {
                    writer = Some(BufWriter::new(file));
                    summary.output_path = Some(output_path.to_path_buf());
                }
                Err(err) => {
                    summary.errors.push(Error {
                        file: Some(output_path.to_path_buf()),
                        message: format!("Failed to create output: {err}"),
                    });
                    return summary;
                }
            }
        }

        if header_ref.is_none() {
            header_ref = content.header.clone();
            summary.header = content.header.clone();
            if let Some(ref header) = header_ref {
                if let Err(err) = write_line(writer.as_mut().unwrap(), header) {
                    summary.errors.push(Error {
                        file: Some(output_path.to_path_buf()),
                        message: format!("Failed to write output: {err}"),
                    });
                    return summary;
                }
            }
        }

        if let (Some(ref header), Some(ref file_header)) = (&header_ref, &content.header) {
            if file_header != header {
                summary.warnings.push(Warning {
                    file: path.clone(),
                    message: "Header mismatch".to_string(),
                });
            }
        }

        for line in content.data_lines {
            if let Err(err) = write_line(writer.as_mut().unwrap(), &line) {
                summary.errors.push(Error {
                    file: Some(output_path.to_path_buf()),
                    message: format!("Failed to write output: {err}"),
                });
                return summary;
            }
            summary.data_lines += 1;
        }

        on_file_processed(file_type, path);
    }

    summary
}

pub fn output_filename(file_type: FileType) -> &'static str {
    match file_type {
        FileType::Bead => "Bead Positions Combined.txt",
        FileType::Motor => "Motor Positions Combined.txt",
    }
}

pub fn collect_warnings(report: &CombineReport) -> Vec<Warning> {
    let mut warnings = Vec::new();
    if let Some(ref bead) = report.bead {
        warnings.extend(bead.warnings.clone());
    }
    if let Some(ref motor) = report.motor {
        warnings.extend(motor.warnings.clone());
    }
    warnings
}

pub fn collect_errors(report: &CombineReport) -> Vec<Error> {
    let mut errors = report.errors.clone();
    if let Some(ref bead) = report.bead {
        errors.extend(bead.errors.clone());
    }
    if let Some(ref motor) = report.motor {
        errors.extend(motor.errors.clone());
    }
    errors
}

fn classify_name(name: &str) -> Option<FileType> {
    if name.starts_with("Bead Positions") {
        Some(FileType::Bead)
    } else if name.starts_with("Motor Positions") {
        Some(FileType::Motor)
    } else {
        None
    }
}

fn is_combined_output(name: &str) -> bool {
    name == "Bead Positions Combined.txt" || name == "Motor Positions Combined.txt"
}

fn sort_paths(paths: &mut Vec<PathBuf>) {
    paths.sort_by_key(|path| file_name_key(path));
}

fn file_name_key(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
}

struct FileContent {
    header: Option<Vec<u8>>,
    data_lines: Vec<Vec<u8>>,
}

fn read_file_content(path: &Path) -> io::Result<FileContent> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();
    let mut header: Option<Vec<u8>> = None;
    let mut data_lines = Vec::new();

    loop {
        buffer.clear();
        let bytes_read = reader.read_until(b'\n', &mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        if buffer.ends_with(&[b'\n']) {
            buffer.pop();
            if buffer.ends_with(&[b'\r']) {
                buffer.pop();
            }
        } else if buffer.ends_with(&[b'\r']) {
            buffer.pop();
        }

        if is_whitespace_line(&buffer) {
            continue;
        }

        if header.is_none() && starts_with_hash(&buffer) {
            header = Some(buffer.clone());
            continue;
        }

        data_lines.push(buffer.clone());
    }

    Ok(FileContent { header, data_lines })
}

fn write_line(writer: &mut BufWriter<File>, line: &[u8]) -> io::Result<()> {
    writer.write_all(line)?;
    writer.write_all(b"\n")?;
    Ok(())
}

fn is_whitespace_line(line: &[u8]) -> bool {
    line.iter().all(|b| b.is_ascii_whitespace())
}

fn starts_with_hash(line: &[u8]) -> bool {
    for byte in line {
        if byte.is_ascii_whitespace() {
            continue;
        }
        return *byte == b'#';
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_file(path: &Path, content: &str) {
        fs::write(path, content).expect("write file");
    }

    #[test]
    fn discover_files_sorts_and_groups() {
        let dir = tempdir().expect("tempdir");
        write_file(
            &dir.path().join("Bead Positions 2.txt"),
            "# H\n1\n",
        );
        write_file(
            &dir.path().join("Bead Positions 1.txt"),
            "# H\n2\n",
        );
        write_file(
            &dir.path().join("Motor Positions 1.txt"),
            "# M\n3\n",
        );

        let discovered = discover_files(dir.path()).expect("discover");
        assert_eq!(discovered.bead_files.len(), 2);
        assert_eq!(discovered.motor_files.len(), 1);
        assert!(discovered.bead_files[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("Bead Positions 1"));
    }

    #[test]
    fn combine_group_uses_first_header_and_concatenates() {
        let dir = tempdir().expect("tempdir");
        write_file(
            &dir.path().join("Bead Positions 1.txt"),
            "# H1\n1\n2\n",
        );
        write_file(
            &dir.path().join("Bead Positions 2.txt"),
            "# H2\n3\n4\n",
        );

        let files = vec![
            dir.path().join("Bead Positions 1.txt"),
            dir.path().join("Bead Positions 2.txt"),
        ];
        let output = dir.path().join("Bead Positions Combined.txt");
        let summary = combine_group(FileType::Bead, &files, &output);

        let content = fs::read_to_string(&output).expect("read output");
        assert!(content.starts_with("# H1\n"));
        assert!(content.contains("1\n"));
        assert!(content.contains("4\n"));
        assert_eq!(summary.data_lines, 4);
        assert_eq!(summary.warnings.len(), 1);
    }

    #[test]
    fn combine_group_warns_on_header_mismatch() {
        let dir = tempdir().expect("tempdir");
        write_file(
            &dir.path().join("Motor Positions 1.txt"),
            "# A\n1\n",
        );
        write_file(
            &dir.path().join("Motor Positions 2.txt"),
            "# B\n2\n",
        );

        let files = vec![
            dir.path().join("Motor Positions 1.txt"),
            dir.path().join("Motor Positions 2.txt"),
        ];
        let output = dir.path().join("Motor Positions Combined.txt");
        let summary = combine_group(FileType::Motor, &files, &output);

        assert_eq!(summary.warnings.len(), 1);
        assert!(summary
            .warnings
            .iter()
            .any(|w| w.message.contains("Header mismatch")));
    }

    #[test]
    fn combine_folder_handles_empty_groups() {
        let dir = tempdir().expect("tempdir");
        let report = combine_folder(dir.path());
        assert_eq!(report.bead_files, 0);
        assert_eq!(report.motor_files, 0);
        assert!(report.bead.is_none());
        assert!(report.motor.is_none());
    }
}
