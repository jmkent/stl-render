use std::path::PathBuf;

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
}

#[derive(Debug, Clone, Parser)]
#[command(name = "stl-render")]
#[command(about = "Render STL files to PNG images")]
#[command(version)]
pub struct Args {
    /// Input STL file path (use - for stdin)
    pub input: PathBuf,

    /// Output PNG path (use - for stdout)
    #[arg(short, long)]
    pub output: PathBuf,

    /// Image width in pixels
    #[arg(long, default_value = "512")]
    pub width: u32,

    /// Image height in pixels
    #[arg(long, default_value = "512")]
    pub height: u32,

    /// View preset: front, back, left, right, top, bottom, iso
    #[arg(long, default_value = "iso")]
    pub view: String,

    /// Camera azimuth angle in degrees (conflicts with --view)
    #[arg(long)]
    pub azimuth: Option<f32>,

    /// Camera elevation angle in degrees (conflicts with --view)
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

pub fn parse_args() -> Result<RenderConfig, RenderError> {
    let args = Args::parse();
    build_config(args).map_err(|e| RenderError::Config(e.to_string()))
}

fn build_config(args: Args) -> Result<RenderConfig, CliError> {
    let view = parse_view(&args)?;
    let aa = parse_aa(&args.aa);
    let background = parse_background(&args.background);
    let background_color = parse_hex_color(&args.background_color);
    let material_color = parse_hex_color(&args.material_color);
    let lighting = parse_lighting(&args.lighting);

    Ok(RenderConfig {
        input: args.input,
        output: args.output,
        width: args.width,
        height: args.height,
        view,
        padding: args.padding,
        aa,
        background,
        background_color,
        material_color,
        lighting,
        metadata_path: args.metadata,
        quiet: args.quiet,
        verbose: args.verbose,
    })
}

fn parse_view(args: &Args) -> Result<ViewConfig, CliError> {
    let has_custom = args.azimuth.is_some() || args.elevation.is_some();

    if has_custom {
        match (args.azimuth, args.elevation) {
            (Some(az), Some(el)) => Ok(ViewConfig::Custom {
                azimuth: az,
                elevation: el,
            }),
            _ => Err(CliError::IncompleteCustomView),
        }
    } else {
        let preset = match args.view.to_lowercase().as_str() {
            "front" => ViewPreset::Front,
            "back" => ViewPreset::Back,
            "left" => ViewPreset::Left,
            "right" => ViewPreset::Right,
            "top" => ViewPreset::Top,
            "bottom" => ViewPreset::Bottom,
            "iso" | "isometric" => ViewPreset::Iso,
            _ => ViewPreset::Iso,
        };
        Ok(ViewConfig::Preset(preset))
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
}
