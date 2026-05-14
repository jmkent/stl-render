use std::path::{Path, PathBuf};

use clap::Parser;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::RenderError;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("cannot use --view with --azimuth or --elevation")]
    ConflictingViewArgs,

    #[error("--azimuth and --elevation must be used together")]
    IncompleteCustomView,

    #[error("cannot use --views with --azimuth or --elevation")]
    ConflictingViewsArgs,

    #[error("cannot use both --view and --views")]
    ConflictingViewFlags,

    #[error("multiple inputs require output directory (path ending with / or existing directory)")]
    MultipleInputsRequireDirectory,

    #[error("stdin input (-) cannot be used with multiple inputs or views")]
    StdinWithBatch,

    #[error("stdout output (-) cannot be used with multiple inputs or views")]
    StdoutWithBatch,
}

#[derive(Debug, Clone, Parser)]
#[command(name = "stl-render")]
#[command(about = "Render STL files to PNG images")]
#[command(version)]
pub struct Args {
    /// Input STL file(s) - supports multiple files and glob patterns (use - for stdin)
    #[arg(required = true)]
    pub inputs: Vec<PathBuf>,

    /// Output PNG path or directory (use - for stdout, use trailing / for directory)
    #[arg(short, long)]
    pub output: PathBuf,

    /// Image width in pixels
    #[arg(long, default_value = "512")]
    pub width: u32,

    /// Image height in pixels
    #[arg(long, default_value = "512")]
    pub height: u32,

    /// View preset: front, back, left, right, top, bottom, iso, print
    #[arg(long, default_value = "iso")]
    pub view: String,

    /// Multiple views (comma-separated): front,back,iso produces multiple outputs
    #[arg(long)]
    pub views: Option<String>,

    /// Camera azimuth angle in degrees (conflicts with --view/--views)
    #[arg(long)]
    pub azimuth: Option<f32>,

    /// Camera elevation angle in degrees (conflicts with --view/--views)
    #[arg(long)]
    pub elevation: Option<f32>,

    /// Padding ratio around model (0.0 - 1.0)
    #[arg(long, default_value = "0.08")]
    pub padding: f32,

    /// Anti-aliasing level: none, 2x, 4x
    #[arg(long, default_value = "2x")]
    pub aa: String,

    /// Background type: transparent, solid
    #[arg(long, default_value = "transparent")]
    pub background: String,

    /// Background color (hex, e.g., #ffffff)
    #[arg(long, default_value = "#ffffff")]
    pub background_color: String,

    /// Material color (hex, e.g., #cccccc)
    #[arg(long, default_value = "#cccccc")]
    pub material_color: String,

    /// Lighting preset: flat, studio, technical
    #[arg(long, default_value = "studio")]
    pub lighting: String,

    /// Write render metadata to JSON file
    #[arg(long)]
    pub metadata: Option<PathBuf>,

    /// Suppress progress output
    #[arg(long)]
    pub quiet: bool,

    /// Show detailed progress
    #[arg(long)]
    pub verbose: bool,

    /// Abort on first error (default: continue and report all errors)
    #[arg(long)]
    pub strict: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ViewPreset {
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
    Iso,
    Print,
    PrintFront,
    PrintLeft,
    PrintRight,
    PrintBack,
    PrintGrid,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ViewConfig {
    Preset(ViewPreset),
    Custom { azimuth: f32, elevation: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AntiAliasing {
    None,
    X2,
    X4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Background {
    Transparent,
    Solid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightingPreset {
    Flat,
    Studio,
    Technical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderConfig {
    pub input: PathBuf,
    pub output: PathBuf,
    pub width: u32,
    pub height: u32,
    pub view: ViewConfig,
    pub padding: f32,
    pub aa: AntiAliasing,
    pub background: Background,
    pub background_color: [u8; 3],
    pub material_color: [u8; 3],
    pub lighting: LightingPreset,
    pub metadata_path: Option<PathBuf>,
    pub quiet: bool,
    pub verbose: bool,
}

#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub inputs: Vec<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub output_file: Option<PathBuf>,
    pub views: Vec<ViewConfig>,
    pub width: u32,
    pub height: u32,
    pub padding: f32,
    pub aa: AntiAliasing,
    pub background: Background,
    pub background_color: [u8; 3],
    pub material_color: [u8; 3],
    pub lighting: LightingPreset,
    pub metadata_path: Option<PathBuf>,
    pub quiet: bool,
    pub verbose: bool,
    pub strict: bool,
}

impl BatchConfig {
    pub fn is_batch_mode(&self) -> bool {
        self.inputs.len() > 1 || self.views.len() > 1
    }

    pub fn iter_jobs(&self) -> impl Iterator<Item = RenderConfig> + '_ {
        self.inputs.iter().flat_map(move |input| {
            self.views.iter().map(move |&view| {
                let output = self.compute_output_path(input, view);
                let metadata_path = self.metadata_path.as_ref().map(|p| {
                    if self.is_batch_mode() {
                        self.compute_metadata_path(input, view)
                    } else {
                        p.clone()
                    }
                });

                RenderConfig {
                    input: input.clone(),
                    output,
                    width: self.width,
                    height: self.height,
                    view,
                    padding: self.padding,
                    aa: self.aa,
                    background: self.background,
                    background_color: self.background_color,
                    material_color: self.material_color,
                    lighting: self.lighting,
                    metadata_path,
                    quiet: self.quiet,
                    verbose: self.verbose,
                }
            })
        })
    }

    fn compute_output_path(&self, input: &Path, view: ViewConfig) -> PathBuf {
        if let Some(ref dir) = self.output_dir {
            let stem = input.file_stem().unwrap_or_default().to_string_lossy();
            if self.views.len() > 1 {
                let view_name = view_config_name(view);
                dir.join(format!("{}.{}.png", stem, view_name))
            } else {
                dir.join(format!("{}.png", stem))
            }
        } else {
            self.output_file.clone().unwrap_or_else(|| PathBuf::from("output.png"))
        }
    }

    fn compute_metadata_path(&self, input: &Path, view: ViewConfig) -> PathBuf {
        if let Some(ref dir) = self.output_dir {
            let stem = input.file_stem().unwrap_or_default().to_string_lossy();
            if self.views.len() > 1 {
                let view_name = view_config_name(view);
                dir.join(format!("{}.{}.json", stem, view_name))
            } else {
                dir.join(format!("{}.json", stem))
            }
        } else {
            self.metadata_path.clone().unwrap_or_else(|| PathBuf::from("metadata.json"))
        }
    }
}

fn view_config_name(view: ViewConfig) -> String {
    match view {
        ViewConfig::Preset(preset) => view_preset_name(preset).to_string(),
        ViewConfig::Custom { azimuth, elevation } => format!("custom_{}_{}", azimuth as i32, elevation as i32),
    }
}

fn view_preset_name(preset: ViewPreset) -> &'static str {
    match preset {
        ViewPreset::Front => "front",
        ViewPreset::Back => "back",
        ViewPreset::Left => "left",
        ViewPreset::Right => "right",
        ViewPreset::Top => "top",
        ViewPreset::Bottom => "bottom",
        ViewPreset::Iso => "iso",
        ViewPreset::Print => "print",
        ViewPreset::PrintFront => "print-front",
        ViewPreset::PrintLeft => "print-left",
        ViewPreset::PrintRight => "print-right",
        ViewPreset::PrintBack => "print-back",
        ViewPreset::PrintGrid => "print-grid",
    }
}

pub fn parse_args() -> Result<BatchConfig, RenderError> {
    let args = Args::parse();
    build_batch_config(args).map_err(|e| RenderError::Config(e.to_string()))
}

fn build_batch_config(args: Args) -> Result<BatchConfig, CliError> {
    let has_custom_angles = args.azimuth.is_some() || args.elevation.is_some();
    let has_views_flag = args.views.is_some();
    let has_explicit_view = args.view != "iso";
    let is_stdin = args.inputs.len() == 1 && args.inputs[0].to_str() == Some("-");
    let is_stdout = args.output.to_str() == Some("-");

    // Validate conflicting options
    if has_custom_angles && has_views_flag {
        return Err(CliError::ConflictingViewsArgs);
    }
    if has_custom_angles && has_explicit_view {
        return Err(CliError::ConflictingViewArgs);
    }
    if has_views_flag && has_explicit_view {
        return Err(CliError::ConflictingViewFlags);
    }

    // Parse views
    let views: Vec<ViewConfig> = if has_custom_angles {
        match (args.azimuth, args.elevation) {
            (Some(az), Some(el)) => {
                vec![ViewConfig::Custom { azimuth: az, elevation: el }]
            }
            _ => return Err(CliError::IncompleteCustomView),
        }
    } else if let Some(ref views_str) = args.views {
        parse_views_list(views_str).into_iter().map(ViewConfig::Preset).collect()
    } else {
        vec![ViewConfig::Preset(parse_view_preset(&args.view))]
    };

    let is_batch = args.inputs.len() > 1 || views.len() > 1;

    // Validate stdin/stdout with batch mode
    if is_stdin && is_batch {
        return Err(CliError::StdinWithBatch);
    }
    if is_stdout && is_batch {
        return Err(CliError::StdoutWithBatch);
    }

    // Determine output mode (file vs directory)
    let (output_dir, output_file) = if is_stdout {
        (None, Some(args.output.clone()))
    } else if is_batch {
        let path = &args.output;
        let is_dir = path.to_str().map(|s| s.ends_with('/')).unwrap_or(false)
            || path.is_dir();
        if !is_dir {
            return Err(CliError::MultipleInputsRequireDirectory);
        }
        (Some(path.clone()), None)
    } else {
        (None, Some(args.output.clone()))
    };

    let aa = parse_aa(&args.aa);
    let background = parse_background(&args.background);
    let background_color = parse_hex_color(&args.background_color);
    let material_color = parse_hex_color(&args.material_color);
    let lighting = parse_lighting(&args.lighting);

    Ok(BatchConfig {
        inputs: args.inputs,
        output_dir,
        output_file,
        views,
        width: args.width,
        height: args.height,
        padding: args.padding,
        aa,
        background,
        background_color,
        material_color,
        lighting,
        metadata_path: args.metadata,
        quiet: args.quiet,
        verbose: args.verbose,
        strict: args.strict,
    })
}

fn parse_views_list(s: &str) -> Vec<ViewPreset> {
    s.split(',')
        .map(|v| parse_view_preset(v.trim()))
        .collect()
}

fn parse_view_preset(s: &str) -> ViewPreset {
    match s.to_lowercase().as_str() {
        "front" => ViewPreset::Front,
        "back" => ViewPreset::Back,
        "left" => ViewPreset::Left,
        "right" => ViewPreset::Right,
        "top" => ViewPreset::Top,
        "bottom" => ViewPreset::Bottom,
        "iso" | "isometric" => ViewPreset::Iso,
        "print" | "bed" => ViewPreset::Print,
        "print-front" | "printfront" => ViewPreset::PrintFront,
        "print-left" | "printleft" => ViewPreset::PrintLeft,
        "print-right" | "printright" => ViewPreset::PrintRight,
        "print-back" | "printback" => ViewPreset::PrintBack,
        "print-grid" | "printgrid" => ViewPreset::PrintGrid,
        _ => ViewPreset::Iso,
    }
}

fn parse_aa(s: &str) -> AntiAliasing {
    match s.to_lowercase().as_str() {
        "none" | "1x" => AntiAliasing::None,
        "4x" => AntiAliasing::X4,
        _ => AntiAliasing::X2,
    }
}

fn parse_background(s: &str) -> Background {
    match s.to_lowercase().as_str() {
        "solid" => Background::Solid,
        _ => Background::Transparent,
    }
}

fn parse_lighting(s: &str) -> LightingPreset {
    match s.to_lowercase().as_str() {
        "flat" => LightingPreset::Flat,
        "technical" => LightingPreset::Technical,
        _ => LightingPreset::Studio,
    }
}

fn parse_hex_color(s: &str) -> [u8; 3] {
    let s = s.trim_start_matches('#');
    if s.len() == 6
        && let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&s[0..2], 16),
            u8::from_str_radix(&s[2..4], 16),
            u8::from_str_radix(&s[4..6], 16),
        )
    {
        return [r, g, b];
    }
    [204, 204, 204] // default gray
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_args(args: &[&str]) -> Args {
        Args::parse_from(args)
    }

    #[test]
    fn test_parse_minimal_args() {
        let args = make_args(&["stl-render", "input.stl", "-o", "out.png"]);
        assert_eq!(args.inputs, vec![PathBuf::from("input.stl")]);
        assert_eq!(args.output, PathBuf::from("out.png"));
        assert_eq!(args.width, 512);
        assert_eq!(args.height, 512);
    }

    #[test]
    fn test_parse_all_flags() {
        let args = make_args(&[
            "stl-render",
            "model.stl",
            "-o", "render.png",
            "--width", "1024",
            "--height", "768",
            "--view", "front",
            "--padding", "0.1",
            "--aa", "4x",
            "--background", "solid",
            "--background-color", "#ff0000",
            "--material-color", "#00ff00",
            "--lighting", "flat",
            "--metadata", "meta.json",
            "--quiet",
        ]);
        assert_eq!(args.width, 1024);
        assert_eq!(args.height, 768);
        assert_eq!(args.view, "front");
        assert_eq!(args.padding, 0.1);
        assert_eq!(args.aa, "4x");
        assert_eq!(args.background, "solid");
        assert!(args.quiet);
    }

    #[test]
    fn test_build_batch_config_minimal() {
        let args = make_args(&["stl-render", "test.stl", "-o", "out.png"]);
        let config = build_batch_config(args).unwrap();
        assert_eq!(config.inputs, vec![PathBuf::from("test.stl")]);
        assert_eq!(config.output_file, Some(PathBuf::from("out.png")));
        assert_eq!(config.views, vec![ViewConfig::Preset(ViewPreset::Iso)]);
    }

    #[test]
    fn test_build_batch_config_custom_view() {
        let args = make_args(&[
            "stl-render", "test.stl", "-o", "out.png",
            "--azimuth", "45", "--elevation", "30",
        ]);
        let config = build_batch_config(args).unwrap();
        assert_eq!(config.views, vec![ViewConfig::Custom { azimuth: 45.0, elevation: 30.0 }]);
    }

    #[test]
    fn test_build_batch_config_incomplete_custom_view() {
        let args = make_args(&[
            "stl-render", "test.stl", "-o", "out.png",
            "--azimuth", "45",
        ]);
        let result = build_batch_config(args);
        assert!(matches!(result, Err(CliError::IncompleteCustomView)));
    }

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("#ff0000"), [255, 0, 0]);
        assert_eq!(parse_hex_color("00ff00"), [0, 255, 0]);
        assert_eq!(parse_hex_color("#0000FF"), [0, 0, 255]);
        assert_eq!(parse_hex_color("invalid"), [204, 204, 204]);
    }

    #[test]
    fn test_parse_aa() {
        assert_eq!(parse_aa("none"), AntiAliasing::None);
        assert_eq!(parse_aa("2x"), AntiAliasing::X2);
        assert_eq!(parse_aa("4x"), AntiAliasing::X4);
    }

    #[test]
    fn test_parse_background() {
        assert_eq!(parse_background("transparent"), Background::Transparent);
        assert_eq!(parse_background("solid"), Background::Solid);
    }

    #[test]
    fn test_parse_view_presets() {
        for (name, expected) in [
            ("front", ViewPreset::Front),
            ("back", ViewPreset::Back),
            ("left", ViewPreset::Left),
            ("right", ViewPreset::Right),
            ("top", ViewPreset::Top),
            ("bottom", ViewPreset::Bottom),
            ("iso", ViewPreset::Iso),
            ("print", ViewPreset::Print),
            ("bed", ViewPreset::Print),
        ] {
            let args = make_args(&["stl-render", "t.stl", "-o", "o.png", "--view", name]);
            let config = build_batch_config(args).unwrap();
            assert_eq!(config.views, vec![ViewConfig::Preset(expected)]);
        }
    }

    #[test]
    fn test_parse_views_list() {
        let args = make_args(&["stl-render", "t.stl", "-o", "out/", "--views", "front,back,iso"]);
        let config = build_batch_config(args).unwrap();
        assert_eq!(config.views, vec![
            ViewConfig::Preset(ViewPreset::Front),
            ViewConfig::Preset(ViewPreset::Back),
            ViewConfig::Preset(ViewPreset::Iso),
        ]);
    }

    #[test]
    fn test_batch_mode_requires_directory() {
        let args = make_args(&["stl-render", "a.stl", "b.stl", "-o", "out.png"]);
        let result = build_batch_config(args);
        assert!(matches!(result, Err(CliError::MultipleInputsRequireDirectory)));
    }

    #[test]
    fn test_multiple_views_requires_directory() {
        let args = make_args(&["stl-render", "t.stl", "-o", "out.png", "--views", "front,back"]);
        let result = build_batch_config(args);
        assert!(matches!(result, Err(CliError::MultipleInputsRequireDirectory)));
    }

    #[test]
    fn test_batch_mode_with_directory() {
        let args = make_args(&["stl-render", "a.stl", "b.stl", "-o", "output/"]);
        let config = build_batch_config(args).unwrap();
        assert_eq!(config.inputs.len(), 2);
        assert_eq!(config.output_dir, Some(PathBuf::from("output/")));
    }

    #[test]
    fn test_strict_flag() {
        let args = make_args(&["stl-render", "t.stl", "-o", "out.png", "--strict"]);
        let config = build_batch_config(args).unwrap();
        assert!(config.strict);
    }
}
