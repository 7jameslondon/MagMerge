use std::env;
use std::io::{self, Write};

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    let exit_code = magmerge::cli::run_cli(&args, &mut stdout, &mut stderr);
    let _ = stdout.flush();
    let _ = stderr.flush();
    std::process::exit(exit_code);
}
