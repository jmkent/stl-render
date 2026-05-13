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
