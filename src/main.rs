use std::fs;
use std::process::ExitCode;

use stl_render::{cli, OutputError, RenderError};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            e.exit_code()
        }
    }
}

fn run() -> Result<(), RenderError> {
    let batch_config = cli::parse_args()?;

    // Create output directory if needed
    if let Some(ref dir) = batch_config.output_dir
        && !dir.exists()
    {
        fs::create_dir_all(dir).map_err(|e| {
            RenderError::Output(OutputError::Io(e))
        })?;
    }

    let is_batch = batch_config.is_batch_mode();
    let strict = batch_config.strict;
    let quiet = batch_config.quiet;
    let verbose = batch_config.verbose;

    let jobs: Vec<_> = batch_config.iter_jobs().collect();
    let total = jobs.len();

    let mut success_count = 0;
    let mut errors: Vec<(String, RenderError)> = Vec::new();

    for (idx, config) in jobs.into_iter().enumerate() {
        let input_name = config.input.display().to_string();
        let output_name = config.output.display().to_string();

        if verbose && is_batch {
            eprintln!("[{}/{}] {} -> {}", idx + 1, total, input_name, output_name);
        }

        match stl_render::render(&config) {
            Ok(_metadata) => {
                success_count += 1;
            }
            Err(e) => {
                if strict {
                    return Err(e);
                }
                if !quiet {
                    eprintln!("error: {}: {}", input_name, e);
                }
                errors.push((input_name, e));
            }
        }
    }

    // Print summary for batch mode
    if is_batch && !quiet {
        if errors.is_empty() {
            eprintln!("Rendered {} file(s) successfully", success_count);
        } else {
            eprintln!(
                "Rendered {} of {} file(s), {} error(s)",
                success_count,
                total,
                errors.len()
            );
        }
    }

    // Return worst error code
    if let Some((_, worst_error)) = errors.into_iter().next() {
        Err(worst_error)
    } else {
        Ok(())
    }
}
