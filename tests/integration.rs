use std::process::Command;
use tempfile::tempdir;

fn stl_render() -> Command {
    Command::new(env!("CARGO_BIN_EXE_stl-render"))
}

#[test]
fn test_render_cube_to_png() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("cube.png");

    let status = stl_render()
        .args(["fixtures/cube.stl", "-o", output.to_str().unwrap()])
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());

    // Verify it's a valid PNG
    let data = std::fs::read(&output).unwrap();
    assert_eq!(&data[0..8], b"\x89PNG\r\n\x1a\n");
}

#[test]
fn test_render_with_custom_size() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("large.png");

    let status = stl_render()
        .args([
            "fixtures/cube.stl",
            "-o", output.to_str().unwrap(),
            "--width", "256",
            "--height", "256",
        ])
        .status()
        .unwrap();

    assert!(status.success());

    // Read PNG header to verify dimensions
    let img = image::open(&output).unwrap();
    assert_eq!(img.width(), 256);
    assert_eq!(img.height(), 256);
}

#[test]
fn test_missing_input_file_exit_code() {
    let status = stl_render()
        .args(["nonexistent.stl", "-o", "/tmp/out.png"])
        .status()
        .unwrap();

    assert!(!status.success());
    assert_eq!(status.code(), Some(2)); // Input error
}

#[test]
fn test_render_with_metadata() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("out.png");
    let meta = dir.path().join("meta.json");

    let status = stl_render()
        .args([
            "fixtures/cube.stl",
            "-o", output.to_str().unwrap(),
            "--metadata", meta.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    assert!(status.success());
    assert!(meta.exists());

    let content = std::fs::read_to_string(&meta).unwrap();
    assert!(content.contains("\"triangle_count\": 12"));
    assert!(content.contains("\"input_file\""));
}

#[test]
fn test_render_to_stdout() {
    let output = stl_render()
        .args(["fixtures/cube.stl", "-o", "-"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(!output.stdout.is_empty());
    // PNG magic bytes
    assert_eq!(&output.stdout[0..8], b"\x89PNG\r\n\x1a\n");
}

#[test]
fn test_render_sphere() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("sphere.png");

    let status = stl_render()
        .args(["fixtures/sphere.stl", "-o", output.to_str().unwrap()])
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());
}

#[test]
fn test_help_flag() {
    let output = stl_render()
        .arg("--help")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Render STL files to PNG"));
}

#[test]
fn test_version_flag() {
    let output = stl_render()
        .arg("--version")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("stl-render"));
}

#[test]
fn test_determinism() {
    let dir = tempdir().unwrap();
    let output1 = dir.path().join("render1.png");
    let output2 = dir.path().join("render2.png");

    let status1 = stl_render()
        .args(["fixtures/cube.stl", "-o", output1.to_str().unwrap(), "--aa", "none"])
        .status()
        .unwrap();
    assert!(status1.success());

    let status2 = stl_render()
        .args(["fixtures/cube.stl", "-o", output2.to_str().unwrap(), "--aa", "none"])
        .status()
        .unwrap();
    assert!(status2.success());

    let data1 = std::fs::read(&output1).unwrap();
    let data2 = std::fs::read(&output2).unwrap();

    assert_eq!(data1, data2, "Same input should produce identical output");
}

#[test]
fn test_render_produces_visible_content() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("cube.png");

    let status = stl_render()
        .args([
            "fixtures/cube.stl",
            "-o", output.to_str().unwrap(),
            "--background", "transparent",
        ])
        .status()
        .unwrap();

    assert!(status.success());

    let img = image::open(&output).unwrap().into_rgba8();
    let non_transparent: usize = img.pixels().filter(|p| p[3] > 0).count();

    assert!(
        non_transparent > 1000,
        "Rendered image should have visible content: {} non-transparent pixels",
        non_transparent
    );
}

#[test]
fn test_material_color_red() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("red.png");

    let status = stl_render()
        .args([
            "fixtures/cube.stl",
            "-o", output.to_str().unwrap(),
            "--material-color", "#ff0000",
            "--background", "transparent",
        ])
        .status()
        .unwrap();

    assert!(status.success());

    let img = image::open(&output).unwrap().into_rgba8();
    let visible: Vec<_> = img.pixels().filter(|p| p[3] > 0).collect();
    assert!(!visible.is_empty());

    let avg_r: u32 = visible.iter().map(|p| p[0] as u32).sum::<u32>() / visible.len() as u32;
    let avg_b: u32 = visible.iter().map(|p| p[2] as u32).sum::<u32>() / visible.len() as u32;

    assert!(avg_r > avg_b * 2, "Red material should have more R than B: r={}, b={}", avg_r, avg_b);
}

#[test]
fn test_lighting_presets_differ() {
    let dir = tempdir().unwrap();
    let flat = dir.path().join("flat.png");
    let studio = dir.path().join("studio.png");
    let technical = dir.path().join("technical.png");

    for (path, preset) in [(&flat, "flat"), (&studio, "studio"), (&technical, "technical")] {
        let status = stl_render()
            .args([
                "fixtures/cube.stl",
                "-o", path.to_str().unwrap(),
                "--lighting", preset,
                "--aa", "none",
            ])
            .status()
            .unwrap();
        assert!(status.success());
    }

    let flat_data = std::fs::read(&flat).unwrap();
    let studio_data = std::fs::read(&studio).unwrap();
    let technical_data = std::fs::read(&technical).unwrap();

    assert_ne!(flat_data, studio_data, "Flat and studio should differ");
    assert_ne!(studio_data, technical_data, "Studio and technical should differ");
}
