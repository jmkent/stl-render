use std::process::ExitCode;

use stl_render::cli;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            e.exit_code()
        }
    }
}

fn run() -> Result<(), stl_render::RenderError> {
    let config = cli::parse_args()?;
    stl_render::render(&config)?;
    Ok(())
}
