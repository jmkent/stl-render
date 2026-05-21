use std::fs;
use std::process::ExitCode;

use stl_render::{MeshReader, OutputError, RenderError, cli};

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

    // Handle --list-colors: print color palette and exit
    if batch_config.list_colors {
        return list_colors(&batch_config);
    }

    // Create output directory if needed
    if let Some(ref dir) = batch_config.output_dir
        && !dir.exists()
    {
        fs::create_dir_all(dir).map_err(|e| RenderError::Output(OutputError::Io(e)))?;
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

        ensure_parent_dir(&config.output)?;
        if let Some(ref metadata_path) = config.metadata_path {
            ensure_parent_dir(metadata_path)?;
        }

        match stl_render::render(&config) {
            Ok(_metadata) => {
                success_count += 1;
                if is_batch && !quiet {
                    eprintln!("Rendered {} as {} successful", input_name, output_name);
                }
            }
            Err(e) => {
                if is_batch && !quiet {
                    eprintln!("Rendered {} as {} failed", input_name, output_name);
                }
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

    // Return highest severity error (lower exit code = more severe: 1=config, 2=input, 3=output)
    if errors.is_empty() {
        Ok(())
    } else {
        let worst = errors
            .into_iter()
            .min_by_key(|(_, e)| {
                // Convert ExitCode to u8 for comparison (there's no From<ExitCode> for u8)
                match &e {
                    RenderError::Config(_) => 1u8,
                    RenderError::Mesh(_) => 2u8,
                    RenderError::Output(_) => 3u8,
                }
            })
            .map(|(_, e)| e);
        Err(worst.unwrap())
    }
}

fn ensure_parent_dir(path: &std::path::Path) -> Result<(), RenderError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        fs::create_dir_all(parent).map_err(|e| RenderError::Output(OutputError::Io(e)))?;
    }
    Ok(())
}

fn list_colors(batch_config: &cli::BatchConfig) -> Result<(), RenderError> {
    if batch_config.inputs.is_empty() {
        return Err(RenderError::Config("no input files specified".into()));
    }

    for input in &batch_config.inputs {
        let reader = MeshReader::open(&input.path)?;

        if batch_config.inputs.len() > 1 {
            println!("{}:", input.path.display());
        }

        if !reader.has_colors() {
            println!("  (no colors)");
            continue;
        }

        let palette = reader.color_palette();
        for (i, color) in palette.iter().enumerate() {
            println!(
                "  {}: #{:02x}{:02x}{:02x} (alpha: {})",
                i, color[0], color[1], color[2], color[3]
            );
        }
    }

    Ok(())
}
