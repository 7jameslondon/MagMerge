use std::fs;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

pub mod cli;

const BEAD_PREFIX: &str = "Bead Positions";
const MOTOR_PREFIX: &str = "Motor Positions";
const BEAD_OUTPUT: &str = "Bead Positions Combined.txt";
const MOTOR_OUTPUT: &str = "Motor Positions Combined.txt";

#[derive(Debug, Clone)]
pub enum ProgressEvent {
    Discovery {
        bead_files: usize,
        motor_files: usize,
    },
    Combine {
        processed_files: usize,
        total_files: usize,
        file_type: FileType,
        current_file: PathBuf,
    },
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
    discover_files_with_progress(folder, |_, _| {})
}

pub fn discover_files_with_progress<F>(
    folder: &Path,
    mut on_discovery: F,
) -> io::Result<DiscoveredFiles>
where
    F: FnMut(usize, usize),
{
    let mut bead_files = Vec::new();
    let mut motor_files = Vec::new();

    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str());
        if ext != Some("txt") {
            continue;
        }

        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if is_combined_output(&name) {
            continue;
        }

        match classify_name(&name) {
            Some(FileType::Bead) => {
                bead_files.push(path);
                on_discovery(bead_files.len(), motor_files.len());
            }
            Some(FileType::Motor) => {
                motor_files.push(path);
                on_discovery(bead_files.len(), motor_files.len());
            }
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
    F: FnMut(ProgressEvent),
{
    let mut report = CombineReport {
        folder: folder.to_path_buf(),
        bead_files: 0,
        motor_files: 0,
        bead: None,
        motor: None,
        errors: Vec::new(),
    };

    let discovered = match discover_files_with_progress(folder, |bead_files, motor_files| {
        on_progress(ProgressEvent::Discovery {
            bead_files,
            motor_files,
        });
    }) {
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
        on_progress(ProgressEvent::Combine {
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
    let mut buffered_before_header: Vec<Vec<u8>> = Vec::new();
    let mut saw_readable_file = false;

    for path in files {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(err) => {
                summary.errors.push(Error {
                    file: Some(path.clone()),
                    message: format!("Failed to read file: {err}"),
                });
                on_file_processed(file_type, path);
                continue;
            }
        };

        let mut reader = BufReader::new(file);
        let mut buffer = Vec::new();
        let mut file_header: Option<Vec<u8>> = None;
        let mut read_failed = false;

        loop {
            match read_next_line(&mut reader, &mut buffer) {
                Ok(true) => {}
                Ok(false) => break,
                Err(err) => {
                    summary.errors.push(Error {
                        file: Some(path.clone()),
                        message: format!("Failed to read file: {err}"),
                    });
                    read_failed = true;
                    break;
                }
            }

            if is_whitespace_line(&buffer) {
                continue;
            }

            if file_header.is_none() && starts_with_hash(&buffer) {
                file_header = Some(buffer.clone());
                let writer = match ensure_output_writer(&mut writer, output_path, &mut summary) {
                    Some(writer) => writer,
                    None => return summary,
                };

                if header_ref.is_none() {
                    header_ref = file_header.clone();
                    summary.header = file_header.clone();
                    if let Some(ref header) = header_ref {
                        if let Err(err) = write_line(writer, header) {
                            summary.errors.push(Error {
                                file: Some(output_path.to_path_buf()),
                                message: format!("Failed to write output: {err}"),
                            });
                            return summary;
                        }
                    }
                    for line in buffered_before_header.drain(..) {
                        if let Err(err) = write_line(writer, &line) {
                            summary.errors.push(Error {
                                file: Some(output_path.to_path_buf()),
                                message: format!("Failed to write output: {err}"),
                            });
                            return summary;
                        }
                        summary.data_lines += 1;
                    }
                } else if header_ref.as_ref() != file_header.as_ref() {
                    summary.warnings.push(Warning {
                        file: path.clone(),
                        message: "Header mismatch".to_string(),
                    });
                }
                continue;
            }

            if header_ref.is_none() {
                buffered_before_header.push(buffer.clone());
                continue;
            }

            let writer = match ensure_output_writer(&mut writer, output_path, &mut summary) {
                Some(writer) => writer,
                None => return summary,
            };
            if let Err(err) = write_line(writer, &buffer) {
                summary.errors.push(Error {
                    file: Some(output_path.to_path_buf()),
                    message: format!("Failed to write output: {err}"),
                });
                return summary;
            }
            summary.data_lines += 1;
        }

        if read_failed {
            on_file_processed(file_type, path);
            continue;
        }

        saw_readable_file = true;
        on_file_processed(file_type, path);
    }

    if header_ref.is_none() && !buffered_before_header.is_empty() {
        let writer = match ensure_output_writer(&mut writer, output_path, &mut summary) {
            Some(writer) => writer,
            None => return summary,
        };
        for line in buffered_before_header.drain(..) {
            if let Err(err) = write_line(writer, &line) {
                summary.errors.push(Error {
                    file: Some(output_path.to_path_buf()),
                    message: format!("Failed to write output: {err}"),
                });
                return summary;
            }
            summary.data_lines += 1;
        }
    }

    if writer.is_none() && saw_readable_file {
        if ensure_output_writer(&mut writer, output_path, &mut summary).is_none() {
            return summary;
        }
    }

    summary
}

pub fn output_filename(file_type: FileType) -> &'static str {
    match file_type {
        FileType::Bead => BEAD_OUTPUT,
        FileType::Motor => MOTOR_OUTPUT,
    }
}

pub fn format_group_output(summary: Option<&GroupSummary>, label: &str) -> String {
    match summary {
        Some(summary) => {
            let output = summary
                .output_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "(not created)".to_string());
            format!("{label} output: {output} (lines: {})", summary.data_lines)
        }
        None => format!("{label} output: (not created)"),
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
    if name.starts_with(BEAD_PREFIX) {
        Some(FileType::Bead)
    } else if name.starts_with(MOTOR_PREFIX) {
        Some(FileType::Motor)
    } else {
        None
    }
}

fn is_combined_output(name: &str) -> bool {
    name == BEAD_OUTPUT || name == MOTOR_OUTPUT
}

fn sort_paths(paths: &mut Vec<PathBuf>) {
    paths.sort_by(|a, b| file_name_key(a).cmp(file_name_key(b)));
}

fn file_name_key(path: &Path) -> &str {
    path.file_name().and_then(|name| name.to_str()).unwrap_or("")
}

fn ensure_output_writer<'a>(
    writer: &'a mut Option<BufWriter<File>>,
    output_path: &Path,
    summary: &mut GroupSummary,
) -> Option<&'a mut BufWriter<File>> {
    if writer.is_none() {
        match File::create(output_path) {
            Ok(file) => {
                *writer = Some(BufWriter::new(file));
                summary.output_path = Some(output_path.to_path_buf());
            }
            Err(err) => {
                summary.errors.push(Error {
                    file: Some(output_path.to_path_buf()),
                    message: format!("Failed to create output: {err}"),
                });
                return None;
            }
        }
    }
    writer.as_mut()
}

fn read_next_line<R: BufRead>(reader: &mut R, buffer: &mut Vec<u8>) -> io::Result<bool> {
    buffer.clear();
    let bytes_read = reader.read_until(b'\n', buffer)?;
    if bytes_read == 0 {
        return Ok(false);
    }
    trim_line_end(buffer);
    Ok(true)
}

fn trim_line_end(buffer: &mut Vec<u8>) {
    if buffer.ends_with(&[b'\n']) {
        buffer.pop();
        if buffer.ends_with(&[b'\r']) {
            buffer.pop();
        }
    } else if buffer.ends_with(&[b'\r']) {
        buffer.pop();
    }
}

fn write_line<W: Write>(writer: &mut W, line: &[u8]) -> io::Result<()> {
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
