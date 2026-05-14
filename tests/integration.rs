use std::io::Write;
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
            "-o",
            output.to_str().unwrap(),
            "--width",
            "256",
            "--height",
            "256",
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

fn assert_invalid_args(args: &[&str], expected_message: &str) {
    let output = stl_render().args(args).output().unwrap();

    assert!(!output.status.success(), "Command should fail: {:?}", args);
    assert_eq!(
        output.status.code(),
        Some(1),
        "Invalid args should be usage/config errors: {:?}",
        args
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected_message),
        "stderr should contain {:?}: {}",
        expected_message,
        stderr
    );
}

#[test]
fn test_invalid_cli_values_exit_code_1() {
    assert_invalid_args(
        &[
            "fixtures/cube.stl",
            "-o",
            "/tmp/invalid-view.png",
            "--view",
            "definitely-not-a-view",
        ],
        "invalid view",
    );
    assert_invalid_args(
        &[
            "fixtures/cube.stl",
            "-o",
            "/tmp/invalid-views/",
            "--views",
            "front,nope,iso",
        ],
        "invalid view",
    );
    assert_invalid_args(
        &[
            "fixtures/cube.stl",
            "-o",
            "/tmp/invalid-aa.png",
            "--aa",
            "nope",
        ],
        "invalid anti-aliasing",
    );
    assert_invalid_args(
        &[
            "fixtures/cube.stl",
            "-o",
            "/tmp/invalid-background.png",
            "--background",
            "gradient",
        ],
        "invalid background",
    );
    assert_invalid_args(
        &[
            "fixtures/cube.stl",
            "-o",
            "/tmp/invalid-lighting.png",
            "--lighting",
            "dramatic",
        ],
        "invalid lighting",
    );
    assert_invalid_args(
        &[
            "fixtures/cube.stl",
            "-o",
            "/tmp/invalid-material.png",
            "--material-color",
            "xyz",
        ],
        "invalid color",
    );
    assert_invalid_args(
        &[
            "fixtures/cube.stl",
            "-o",
            "/tmp/invalid-background-color.png",
            "--background-color",
            "xyz",
        ],
        "invalid color",
    );
}

#[test]
fn test_render_with_metadata() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("out.png");
    let meta = dir.path().join("meta.json");

    let status = stl_render()
        .args([
            "fixtures/cube.stl",
            "-o",
            output.to_str().unwrap(),
            "--metadata",
            meta.to_str().unwrap(),
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
    let output = stl_render().arg("--help").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Render STL and 3MF files to PNG"));
    assert!(
        stdout.contains("tan, blue-grey"),
        "Help should list material color presets: {}",
        stdout
    );
    assert!(
        stdout.contains("--recursive"),
        "Help should list recursive mode: {}",
        stdout
    );
}

#[test]
fn test_version_flag() {
    let output = stl_render().arg("--version").output().unwrap();

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
        .args([
            "fixtures/cube.stl",
            "-o",
            output1.to_str().unwrap(),
            "--aa",
            "none",
        ])
        .status()
        .unwrap();
    assert!(status1.success());

    let status2 = stl_render()
        .args([
            "fixtures/cube.stl",
            "-o",
            output2.to_str().unwrap(),
            "--aa",
            "none",
        ])
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
            "-o",
            output.to_str().unwrap(),
            "--background",
            "transparent",
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
            "-o",
            output.to_str().unwrap(),
            "--material-color",
            "#ff0000",
            "--background",
            "transparent",
        ])
        .status()
        .unwrap();

    assert!(status.success());

    let img = image::open(&output).unwrap().into_rgba8();
    let visible: Vec<_> = img.pixels().filter(|p| p[3] > 0).collect();
    assert!(!visible.is_empty());

    let avg_r: u32 = visible.iter().map(|p| p[0] as u32).sum::<u32>() / visible.len() as u32;
    let avg_b: u32 = visible.iter().map(|p| p[2] as u32).sum::<u32>() / visible.len() as u32;

    assert!(
        avg_r > avg_b * 2,
        "Red material should have more R than B: r={}, b={}",
        avg_r,
        avg_b
    );
}

#[test]
fn test_material_color_presets_render_successfully() {
    let dir = tempdir().unwrap();

    for color in [
        "tan",
        "blue-grey",
        "TAN",
        "white",
        "black",
        "red",
        "orange",
        "green",
        "blue",
        "grey",
        "gray",
        "silver",
        "#ff0000",
    ] {
        let output = dir
            .path()
            .join(format!("{}.png", color.replace('#', "hex-")));
        let status = stl_render()
            .args([
                "fixtures/cube.stl",
                "-o",
                output.to_str().unwrap(),
                "--material-color",
                color,
                "--aa",
                "none",
            ])
            .status()
            .unwrap();

        assert!(status.success(), "{color} should render successfully");
        assert!(output.exists(), "{color} should write output");
    }
}

#[test]
fn test_lighting_presets_differ() {
    let dir = tempdir().unwrap();
    let flat = dir.path().join("flat.png");
    let studio = dir.path().join("studio.png");
    let technical = dir.path().join("technical.png");

    for (path, preset) in [
        (&flat, "flat"),
        (&studio, "studio"),
        (&technical, "technical"),
    ] {
        let status = stl_render()
            .args([
                "fixtures/cube.stl",
                "-o",
                path.to_str().unwrap(),
                "--lighting",
                preset,
                "--aa",
                "none",
            ])
            .status()
            .unwrap();
        assert!(status.success());
    }

    let flat_data = std::fs::read(&flat).unwrap();
    let studio_data = std::fs::read(&studio).unwrap();
    let technical_data = std::fs::read(&technical).unwrap();

    assert_ne!(flat_data, studio_data, "Flat and studio should differ");
    assert_ne!(
        studio_data, technical_data,
        "Studio and technical should differ"
    );
}

#[test]
fn test_aa_2x_output_dimensions() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("aa2x.png");

    let status = stl_render()
        .args([
            "fixtures/cube.stl",
            "-o",
            output.to_str().unwrap(),
            "--width",
            "256",
            "--height",
            "256",
            "--aa",
            "2x",
        ])
        .status()
        .unwrap();

    assert!(status.success());

    let img = image::open(&output).unwrap();
    assert_eq!(img.width(), 256, "Output should match requested width");
    assert_eq!(img.height(), 256, "Output should match requested height");
}

#[test]
fn test_aa_4x_output_dimensions() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("aa4x.png");

    let status = stl_render()
        .args([
            "fixtures/cube.stl",
            "-o",
            output.to_str().unwrap(),
            "--width",
            "128",
            "--height",
            "128",
            "--aa",
            "4x",
        ])
        .status()
        .unwrap();

    assert!(status.success());

    let img = image::open(&output).unwrap();
    assert_eq!(img.width(), 128);
    assert_eq!(img.height(), 128);
}

#[test]
fn test_aa_none_vs_2x_differ() {
    let dir = tempdir().unwrap();
    let none = dir.path().join("none.png");
    let aa2x = dir.path().join("aa2x.png");

    stl_render()
        .args([
            "fixtures/cube.stl",
            "-o",
            none.to_str().unwrap(),
            "--aa",
            "none",
        ])
        .status()
        .unwrap();

    stl_render()
        .args([
            "fixtures/cube.stl",
            "-o",
            aa2x.to_str().unwrap(),
            "--aa",
            "2x",
        ])
        .status()
        .unwrap();

    let none_data = std::fs::read(&none).unwrap();
    let aa2x_data = std::fs::read(&aa2x).unwrap();

    assert_ne!(
        none_data, aa2x_data,
        "AA should produce different output than no AA"
    );
}

#[test]
fn test_read_stl_from_stdin() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("stdin.png");

    let cube_data = std::fs::read("fixtures/cube.stl").unwrap();

    let mut child = stl_render()
        .args(["-", "-o", output.to_str().unwrap()])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    child.stdin.take().unwrap().write_all(&cube_data).unwrap();
    let status = child.wait().unwrap();

    assert!(status.success());
    assert!(output.exists());

    let img = image::open(&output).unwrap();
    assert_eq!(img.width(), 512);
}

#[test]
fn test_background_transparent_has_alpha() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("transparent.png");

    stl_render()
        .args([
            "fixtures/cube.stl",
            "-o",
            output.to_str().unwrap(),
            "--background",
            "transparent",
        ])
        .status()
        .unwrap();

    let img = image::open(&output).unwrap().into_rgba8();
    let transparent_pixels: usize = img.pixels().filter(|p| p[3] == 0).count();

    assert!(
        transparent_pixels > 1000,
        "Transparent background should have alpha=0 pixels"
    );
}

#[test]
fn test_background_solid_uses_color() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("solid.png");

    stl_render()
        .args([
            "fixtures/cube.stl",
            "-o",
            output.to_str().unwrap(),
            "--background",
            "solid",
            "--background-color",
            "#ff0000",
        ])
        .status()
        .unwrap();

    let img = image::open(&output).unwrap().into_rgba8();

    // Find a corner pixel that should be background
    let corner = img.get_pixel(0, 0);
    assert_eq!(corner[0], 255, "Background should be red");
    assert_eq!(corner[3], 255, "Solid background should have alpha=255");
}

#[test]
fn test_default_background_is_transparent() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("default.png");

    stl_render()
        .args(["fixtures/cube.stl", "-o", output.to_str().unwrap()])
        .status()
        .unwrap();

    let img = image::open(&output).unwrap().into_rgba8();

    // Corner should be transparent (default)
    let corner = img.get_pixel(0, 0);
    assert_eq!(corner[3], 0, "Default background should be transparent");
}

#[test]
fn test_metadata_contains_required_fields() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("out.png");
    let meta = dir.path().join("meta.json");

    stl_render()
        .args([
            "fixtures/cube.stl",
            "-o",
            output.to_str().unwrap(),
            "--metadata",
            meta.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    let content = std::fs::read_to_string(&meta).unwrap();

    assert!(
        content.contains("\"triangle_count\""),
        "Should have triangle_count"
    );
    assert!(content.contains("\"dimensions\""), "Should have dimensions");
    assert!(
        content.contains("\"bounding_box\""),
        "Should have bounding_box"
    );
    assert!(content.contains("\"input_file\""), "Should have input_file");
}

#[test]
fn test_large_file_renders_successfully() {
    // Skip if large fixture doesn't exist (not committed to repo)
    let large_path = std::path::Path::new("fixtures/large_1m.stl");
    if !large_path.exists() {
        eprintln!("Skipping large file test - fixtures/large_1m.stl not found");
        return;
    }

    let dir = tempdir().unwrap();
    let output = dir.path().join("large.png");

    let status = stl_render()
        .args([
            "fixtures/large_1m.stl",
            "-o",
            output.to_str().unwrap(),
            "--aa",
            "none",
        ])
        .status()
        .unwrap();

    assert!(status.success(), "Large file should render successfully");
    assert!(output.exists());

    let img = image::open(&output).unwrap();
    assert_eq!(img.width(), 512);
}

#[test]
fn test_truncated_file_error() {
    let output = stl_render()
        .args(["fixtures/truncated.stl", "-o", "/tmp/truncated.png"])
        .output()
        .unwrap();

    assert!(!output.status.success(), "Truncated file should fail");
    assert_eq!(
        output.status.code(),
        Some(2),
        "Should be input error (code 2)"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("error") || stderr.contains("Error"),
        "Should print error message: {}",
        stderr
    );
}

#[test]
fn test_empty_stl_render_is_input_error() {
    let dir = tempdir().unwrap();
    let output_path = dir.path().join("empty.png");
    let metadata_path = dir.path().join("empty.json");

    let output = stl_render()
        .args([
            "fixtures/empty.stl",
            "-o",
            output_path.to_str().unwrap(),
            "--metadata",
            metadata_path.to_str().unwrap(),
            "--verbose",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success(), "Empty STL should fail to render");
    assert_eq!(
        output.status.code(),
        Some(2),
        "Should be input error (code 2)"
    );
    assert!(!output_path.exists(), "Empty STL should not write a PNG");
    assert!(
        !metadata_path.exists(),
        "Empty STL should not write metadata"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no triangles"),
        "Should explain that there are no triangles: {}",
        stderr
    );
    assert!(
        !stderr.contains("inf") && !stderr.contains("NaN"),
        "Verbose output should not expose invalid bounds: {}",
        stderr
    );
}

#[test]
fn test_verbose_shows_triangle_count() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("verbose.png");

    let result = stl_render()
        .args([
            "fixtures/cube.stl",
            "-o",
            output.to_str().unwrap(),
            "--verbose",
        ])
        .output()
        .unwrap();

    assert!(result.status.success());

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("12 triangles"),
        "Verbose should show triangle count: {}",
        stderr
    );
}

// Batch mode tests

#[test]
fn test_batch_multiple_inputs_to_directory() {
    let dir = tempdir().unwrap();
    let outdir = dir.path().join("output");
    std::fs::create_dir(&outdir).unwrap();

    let status = stl_render()
        .args([
            "fixtures/cube.stl",
            "fixtures/sphere.stl",
            "-o",
            &format!("{}/", outdir.display()),
        ])
        .status()
        .unwrap();

    assert!(status.success());
    assert!(outdir.join("cube.png").exists(), "cube.png should exist");
    assert!(
        outdir.join("sphere.png").exists(),
        "sphere.png should exist"
    );
}

#[test]
fn test_batch_multiple_views() {
    let dir = tempdir().unwrap();
    let outdir = dir.path().join("output");
    std::fs::create_dir(&outdir).unwrap();

    let status = stl_render()
        .args([
            "fixtures/cube.stl",
            "-o",
            &format!("{}/", outdir.display()),
            "--views",
            "front,back,iso",
        ])
        .status()
        .unwrap();

    assert!(status.success());
    assert!(
        outdir.join("cube.front.png").exists(),
        "cube.front.png should exist"
    );
    assert!(
        outdir.join("cube.back.png").exists(),
        "cube.back.png should exist"
    );
    assert!(
        outdir.join("cube.iso.png").exists(),
        "cube.iso.png should exist"
    );
}

#[test]
fn test_batch_multiple_inputs_multiple_views() {
    let dir = tempdir().unwrap();
    let outdir = dir.path().join("output");
    std::fs::create_dir(&outdir).unwrap();

    let status = stl_render()
        .args([
            "fixtures/cube.stl",
            "fixtures/sphere.stl",
            "-o",
            &format!("{}/", outdir.display()),
            "--views",
            "front,iso",
        ])
        .status()
        .unwrap();

    assert!(status.success());
    assert!(outdir.join("cube.front.png").exists());
    assert!(outdir.join("cube.iso.png").exists());
    assert!(outdir.join("sphere.front.png").exists());
    assert!(outdir.join("sphere.iso.png").exists());
}

#[test]
fn test_batch_continues_on_error() {
    let dir = tempdir().unwrap();
    let outdir = dir.path().join("output");
    std::fs::create_dir(&outdir).unwrap();

    let output = stl_render()
        .args([
            "fixtures/cube.stl",
            "fixtures/nonexistent.stl",
            "fixtures/sphere.stl",
            "-o",
            &format!("{}/", outdir.display()),
        ])
        .output()
        .unwrap();

    // Should fail overall due to nonexistent file
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));

    // But valid files should still be rendered
    assert!(
        outdir.join("cube.png").exists(),
        "cube.png should still be rendered"
    );
    assert!(
        outdir.join("sphere.png").exists(),
        "sphere.png should still be rendered"
    );
}

#[test]
fn test_batch_strict_aborts_on_first_error() {
    let dir = tempdir().unwrap();
    let outdir = dir.path().join("output");
    std::fs::create_dir(&outdir).unwrap();

    let output = stl_render()
        .args([
            "fixtures/nonexistent.stl",
            "fixtures/cube.stl",
            "-o",
            &format!("{}/", outdir.display()),
            "--strict",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    // With strict mode, cube.stl should NOT be rendered after the error
    assert!(
        !outdir.join("cube.png").exists(),
        "cube.png should not exist with --strict"
    );
}

#[test]
fn test_batch_summary_output() {
    let dir = tempdir().unwrap();
    let outdir = dir.path().join("output");
    std::fs::create_dir(&outdir).unwrap();

    let output = stl_render()
        .args([
            "fixtures/cube.stl",
            "fixtures/sphere.stl",
            "-o",
            &format!("{}/", outdir.display()),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("2 file(s)"),
        "Should show summary: {}",
        stderr
    );
}

#[test]
fn test_batch_reports_each_conversion() {
    let dir = tempdir().unwrap();
    let outdir = dir.path().join("output");
    std::fs::create_dir(&outdir).unwrap();

    let output = stl_render()
        .args([
            "fixtures/cube.stl",
            "fixtures/sphere.stl",
            "-o",
            &format!("{}/", outdir.display()),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!(
            "Rendered fixtures/cube.stl as {} successful",
            outdir.join("cube.png").display()
        )),
        "Should show cube conversion line: {}",
        stderr
    );
    assert!(
        stderr.contains(&format!(
            "Rendered fixtures/sphere.stl as {} successful",
            outdir.join("sphere.png").display()
        )),
        "Should show sphere conversion line: {}",
        stderr
    );
}

#[test]
fn test_batch_reports_failed_conversion() {
    let dir = tempdir().unwrap();
    let outdir = dir.path().join("output");
    std::fs::create_dir(&outdir).unwrap();

    let output = stl_render()
        .args([
            "fixtures/cube.stl",
            "fixtures/nonexistent.stl",
            "-o",
            &format!("{}/", outdir.display()),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!(
            "Rendered fixtures/cube.stl as {} successful",
            outdir.join("cube.png").display()
        )),
        "Should show successful conversion line: {}",
        stderr
    );
    assert!(
        stderr.contains(&format!(
            "Rendered fixtures/nonexistent.stl as {} failed",
            outdir.join("nonexistent.png").display()
        )),
        "Should show failed conversion line: {}",
        stderr
    );
}

#[test]
fn test_batch_recursive_renders_nested_directories() {
    let dir = tempdir().unwrap();
    let input_root = dir.path().join("models");
    let nested = input_root.join("nested");
    let outdir = dir.path().join("output");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::create_dir(&outdir).unwrap();
    std::fs::copy("fixtures/cube.stl", input_root.join("cube.stl")).unwrap();
    std::fs::copy("fixtures/cube.obj", input_root.join("cube.OBJ")).unwrap();
    std::fs::copy("fixtures/cube.3mf", input_root.join("cube.3MF")).unwrap();
    std::fs::copy("fixtures/sphere.obj", nested.join("sphere.obj")).unwrap();
    std::fs::copy("fixtures/sphere.3mf", nested.join("sphere.3mf")).unwrap();
    std::fs::write(nested.join("ignore.txt"), "not an stl").unwrap();

    let output = stl_render()
        .args([
            input_root.to_str().unwrap(),
            "-o",
            &format!("{}/", outdir.display()),
            "--recursive",
            "--aa",
            "none",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(outdir.join("cube.stl.png").exists());
    assert!(outdir.join("cube.OBJ.png").exists());
    assert!(outdir.join("cube.3MF.png").exists());
    assert!(outdir.join("nested/sphere.obj.png").exists());
    assert!(outdir.join("nested/sphere.3mf.png").exists());
    assert!(!outdir.join("nested/ignore.png").exists());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!(
            "Rendered {} as {} successful",
            input_root.join("cube.stl").display(),
            outdir.join("cube.stl.png").display()
        )),
        "Should show root conversion line: {}",
        stderr
    );
    assert!(
        stderr.contains(&format!(
            "Rendered {} as {} successful",
            input_root.join("cube.OBJ").display(),
            outdir.join("cube.OBJ.png").display()
        )),
        "Should show uppercase OBJ conversion line: {}",
        stderr
    );
    assert!(
        stderr.contains(&format!(
            "Rendered {} as {} successful",
            input_root.join("cube.3MF").display(),
            outdir.join("cube.3MF.png").display()
        )),
        "Should show uppercase 3MF conversion line: {}",
        stderr
    );
    assert!(
        stderr.contains(&format!(
            "Rendered {} as {} successful",
            nested.join("sphere.obj").display(),
            outdir.join("nested/sphere.obj.png").display()
        )),
        "Should show nested OBJ conversion line: {}",
        stderr
    );
    assert!(
        stderr.contains(&format!(
            "Rendered {} as {} successful",
            nested.join("sphere.3mf").display(),
            outdir.join("nested/sphere.3mf.png").display()
        )),
        "Should show nested 3MF conversion line: {}",
        stderr
    );
}

#[test]
fn test_batch_directory_input_renders_supported_formats_without_recursive() {
    let dir = tempdir().unwrap();
    let input_root = dir.path().join("models");
    let nested = input_root.join("nested");
    let outdir = dir.path().join("output");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::create_dir(&outdir).unwrap();
    std::fs::copy("fixtures/cube.stl", input_root.join("cube.stl")).unwrap();
    std::fs::copy("fixtures/cube.obj", input_root.join("cube.OBJ")).unwrap();
    std::fs::copy("fixtures/cube.3mf", input_root.join("cube.3mf")).unwrap();
    std::fs::copy("fixtures/sphere.stl", nested.join("sphere.stl")).unwrap();

    let output = stl_render()
        .args([
            input_root.to_str().unwrap(),
            "-o",
            &format!("{}/", outdir.display()),
            "--aa",
            "none",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(outdir.join("cube.stl.png").exists());
    assert!(outdir.join("cube.OBJ.png").exists());
    assert!(outdir.join("cube.3mf.png").exists());
    assert!(!outdir.join("nested/sphere.png").exists());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!(
            "Rendered {} as {} successful",
            input_root.join("cube.stl").display(),
            outdir.join("cube.stl.png").display()
        )),
        "Should show STL conversion line: {}",
        stderr
    );
    assert!(
        stderr.contains(&format!(
            "Rendered {} as {} successful",
            input_root.join("cube.OBJ").display(),
            outdir.join("cube.OBJ.png").display()
        )),
        "Should show uppercase OBJ conversion line: {}",
        stderr
    );
    assert!(
        stderr.contains(&format!(
            "Rendered {} as {} successful",
            input_root.join("cube.3mf").display(),
            outdir.join("cube.3mf.png").display()
        )),
        "Should show 3MF conversion line: {}",
        stderr
    );
}

#[test]
fn test_batch_recursive_renders_multiple_views() {
    let dir = tempdir().unwrap();
    let input_root = dir.path().join("models");
    let nested = input_root.join("nested");
    let outdir = dir.path().join("output");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::create_dir(&outdir).unwrap();
    std::fs::copy("fixtures/cube.stl", nested.join("cube.stl")).unwrap();

    let status = stl_render()
        .args([
            input_root.to_str().unwrap(),
            "-o",
            &format!("{}/", outdir.display()),
            "--recursive",
            "--views",
            "front,iso",
            "--aa",
            "none",
        ])
        .status()
        .unwrap();

    assert!(status.success());
    assert!(outdir.join("nested/cube.front.png").exists());
    assert!(outdir.join("nested/cube.iso.png").exists());
}

#[test]
fn test_batch_requires_directory_for_multiple_inputs() {
    let output = stl_render()
        .args([
            "fixtures/cube.stl",
            "fixtures/sphere.stl",
            "-o",
            "output.png",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1)); // Config error
}

#[test]
fn test_batch_requires_directory_for_multiple_views() {
    let output = stl_render()
        .args([
            "fixtures/cube.stl",
            "-o",
            "output.png",
            "--views",
            "front,back",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1)); // Config error
}

// M11: Print View Presets tests

#[test]
fn test_print_front_view() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("print-front.png");

    let status = stl_render()
        .args([
            "fixtures/cube.stl",
            "-o",
            output.to_str().unwrap(),
            "--view",
            "print-front",
        ])
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());

    let img = image::open(&output).unwrap().into_rgba8();
    let non_transparent: usize = img.pixels().filter(|p| p[3] > 0).count();
    assert!(non_transparent > 1000, "Should have visible content");
}

#[test]
fn test_print_left_view() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("print-left.png");

    let status = stl_render()
        .args([
            "fixtures/cube.stl",
            "-o",
            output.to_str().unwrap(),
            "--view",
            "print-left",
        ])
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());
}

#[test]
fn test_print_right_view() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("print-right.png");

    let status = stl_render()
        .args([
            "fixtures/cube.stl",
            "-o",
            output.to_str().unwrap(),
            "--view",
            "print-right",
        ])
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());
}

#[test]
fn test_print_back_view() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("print-back.png");

    let status = stl_render()
        .args([
            "fixtures/cube.stl",
            "-o",
            output.to_str().unwrap(),
            "--view",
            "print-back",
        ])
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());
}

#[test]
fn test_print_views_differ() {
    // Use single_triangle which is asymmetric (cube/sphere/cylinder are too symmetric)
    let dir = tempdir().unwrap();
    let front = dir.path().join("front.png");
    let left = dir.path().join("left.png");
    let right = dir.path().join("right.png");
    let back = dir.path().join("back.png");

    for (path, view) in [
        (&front, "print-front"),
        (&left, "print-left"),
        (&right, "print-right"),
        (&back, "print-back"),
    ] {
        stl_render()
            .args([
                "fixtures/single_triangle.stl",
                "-o",
                path.to_str().unwrap(),
                "--view",
                view,
                "--aa",
                "none",
            ])
            .status()
            .unwrap();
    }

    let front_data = std::fs::read(&front).unwrap();
    let left_data = std::fs::read(&left).unwrap();
    let right_data = std::fs::read(&right).unwrap();
    let back_data = std::fs::read(&back).unwrap();

    assert_ne!(front_data, left_data, "Front and left should differ");
    assert_ne!(front_data, right_data, "Front and right should differ");
    assert_ne!(front_data, back_data, "Front and back should differ");
    assert_ne!(left_data, back_data, "Left and back should differ");
}

#[test]
fn test_print_grid_produces_composite() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("grid.png");

    let status = stl_render()
        .args([
            "fixtures/cube.stl",
            "-o",
            output.to_str().unwrap(),
            "--view",
            "print-grid",
            "--width",
            "512",
            "--height",
            "512",
        ])
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());

    // Verify dimensions are correct
    let img = image::open(&output).unwrap();
    assert_eq!(img.width(), 512, "Grid width should match requested");
    assert_eq!(img.height(), 512, "Grid height should match requested");
}

#[test]
fn test_print_grid_has_visible_content_in_all_quadrants() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("grid.png");

    stl_render()
        .args([
            "fixtures/cube.stl",
            "-o",
            output.to_str().unwrap(),
            "--view",
            "print-grid",
            "--width",
            "256",
            "--height",
            "256",
            "--background",
            "transparent",
        ])
        .status()
        .unwrap();

    let img = image::open(&output).unwrap().into_rgba8();

    // Check each quadrant has some non-transparent pixels
    let quadrants = [
        (0, 0, 128, 128),     // top-left
        (128, 0, 256, 128),   // top-right
        (0, 128, 128, 256),   // bottom-left
        (128, 128, 256, 256), // bottom-right
    ];

    for (x1, y1, x2, y2) in quadrants {
        let mut non_transparent = 0;
        for y in y1..y2 {
            for x in x1..x2 {
                if img.get_pixel(x, y)[3] > 0 {
                    non_transparent += 1;
                }
            }
        }
        assert!(
            non_transparent > 100,
            "Quadrant ({},{}) to ({},{}) should have visible content, found {} pixels",
            x1,
            y1,
            x2,
            y2,
            non_transparent
        );
    }
}

fn assert_print_grid_from_stdin(input_path: &str) {
    let dir = tempdir().unwrap();
    let output = dir.path().join("grid.png");
    let stl_data = std::fs::read(input_path).unwrap();

    let mut child = stl_render()
        .args([
            "-",
            "-o",
            output.to_str().unwrap(),
            "--view",
            "print-grid",
            "--width",
            "256",
            "--height",
            "256",
        ])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    child.stdin.take().unwrap().write_all(&stl_data).unwrap();
    let status = child.wait().unwrap();

    assert!(
        status.success(),
        "print-grid should read {} from stdin",
        input_path
    );
    assert!(output.exists());

    let img = image::open(&output).unwrap();
    assert_eq!(img.width(), 256);
    assert_eq!(img.height(), 256);
}

#[test]
fn test_print_grid_reads_binary_stl_from_stdin() {
    assert_print_grid_from_stdin("fixtures/cube.stl");
}

#[test]
fn test_print_grid_reads_ascii_stl_from_stdin() {
    assert_print_grid_from_stdin("fixtures/cube_ascii.stl");
}

#[test]
fn test_print_views_in_batch_mode() {
    let dir = tempdir().unwrap();
    let outdir = dir.path().join("output");
    std::fs::create_dir(&outdir).unwrap();

    let status = stl_render()
        .args([
            "fixtures/cube.stl",
            "-o",
            &format!("{}/", outdir.display()),
            "--views",
            "print-front,print-left,print-right,print-back",
        ])
        .status()
        .unwrap();

    assert!(status.success());
    assert!(outdir.join("cube.print-front.png").exists());
    assert!(outdir.join("cube.print-left.png").exists());
    assert!(outdir.join("cube.print-right.png").exists());
    assert!(outdir.join("cube.print-back.png").exists());
}

// ============================================================================
// Library API Tests (M13)
// ============================================================================

#[test]
fn test_render_config_builder() {
    use stl_render::{AntiAliasing, LightingPreset, RenderConfigBuilder, ViewPreset};

    let config = RenderConfigBuilder::new("model.stl", "output.png")
        .width(1024)
        .height(768)
        .view(ViewPreset::Print)
        .aa(AntiAliasing::X4)
        .material_color([193, 154, 107])
        .lighting(LightingPreset::Flat)
        .padding(0.1)
        .build();

    assert_eq!(config.width, 1024);
    assert_eq!(config.height, 768);
    assert_eq!(config.material_color, [193, 154, 107]);
    assert_eq!(config.padding, 0.1);
}

#[test]
fn test_render_config_builder_defaults() {
    use stl_render::{
        AntiAliasing, Background, LightingPreset, RenderConfigBuilder, ViewConfig, ViewPreset,
    };

    let config = RenderConfigBuilder::new("input.stl", "output.png").build();

    assert_eq!(config.width, 512);
    assert_eq!(config.height, 512);
    assert_eq!(config.view, ViewConfig::Preset(ViewPreset::Iso));
    assert_eq!(config.aa, AntiAliasing::X2);
    assert_eq!(config.background, Background::Transparent);
    assert_eq!(config.lighting, LightingPreset::Studio);
    assert_eq!(config.padding, 0.08);
}

#[test]
fn test_render_config_builder_size_helper() {
    use stl_render::RenderConfigBuilder;

    let config = RenderConfigBuilder::new("input.stl", "output.png")
        .size(256)
        .build();

    assert_eq!(config.width, 256);
    assert_eq!(config.height, 256);
}

#[test]
fn test_render_config_builder_custom_view() {
    use stl_render::{RenderConfigBuilder, ViewConfig};

    let config = RenderConfigBuilder::new("input.stl", "output.png")
        .custom_view(45.0, 30.0)
        .build();

    assert_eq!(
        config.view,
        ViewConfig::Custom {
            azimuth: 45.0,
            elevation: 30.0
        }
    );
}

#[test]
fn test_render_config_builder_solid_background() {
    use stl_render::{Background, RenderConfigBuilder};

    let config = RenderConfigBuilder::new("input.stl", "output.png")
        .solid_background([32, 32, 32])
        .build();

    assert_eq!(config.background, Background::Solid);
    assert_eq!(config.background_color, [32, 32, 32]);
}

#[test]
fn test_render_to_image_returns_image_and_metadata() {
    use stl_render::{RenderConfigBuilder, render_to_image};

    let config = RenderConfigBuilder::new("fixtures/cube.stl", "-")
        .size(128)
        .build();

    let (image, metadata) = render_to_image(&config).unwrap();

    assert_eq!(image.width(), 128);
    assert_eq!(image.height(), 128);
    assert_eq!(metadata.triangle_count, 12);
    assert!(metadata.input_file.contains("cube.stl"));
}

#[test]
fn test_render_to_image_with_print_grid() {
    use stl_render::{RenderConfigBuilder, ViewPreset, render_to_image};

    let config = RenderConfigBuilder::new("fixtures/cube.stl", "-")
        .view(ViewPreset::PrintGrid)
        .size(256)
        .build();

    let (image, metadata) = render_to_image(&config).unwrap();

    assert_eq!(image.width(), 256);
    assert_eq!(image.height(), 256);
    assert_eq!(metadata.triangle_count, 12);
}

#[test]
fn test_library_exports_all_types() {
    // This test verifies that all expected types are exported from the library
    use stl_render::{
        AntiAliasing, Background, LightingPreset, MeshReader, Tmf3Reader, ViewPreset,
    };

    // Just verify they exist and can be named
    let _: fn() -> AntiAliasing = || AntiAliasing::X2;
    let _: fn() -> Background = || Background::Transparent;
    let _: fn() -> LightingPreset = || LightingPreset::Studio;
    let _: fn() -> ViewPreset = || ViewPreset::Iso;

    // Verify mesh reader types are exported
    let _ = |path: &std::path::Path| MeshReader::open(path);
    let _ = |path: &std::path::Path| Tmf3Reader::open(path);
    let _ = |path: &std::path::Path| stl_render::ObjReader::open(path);
}

// =============================================================================
// 3MF Format Tests
// =============================================================================

#[test]
fn test_render_3mf_cube() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("cube.png");

    let status = Command::new(env!("CARGO_BIN_EXE_stl-render"))
        .args(["fixtures/cube.3mf", "-o"])
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());

    let img = image::open(&output).unwrap();
    assert_eq!(img.width(), 512);
    assert_eq!(img.height(), 512);
}

#[test]
fn test_render_3mf_sphere() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("sphere.png");

    let status = Command::new(env!("CARGO_BIN_EXE_stl-render"))
        .args(["fixtures/sphere.3mf", "-o"])
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());
}

#[test]
fn test_render_3mf_multi_object() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("multi.png");

    let status = Command::new(env!("CARGO_BIN_EXE_stl-render"))
        .args(["fixtures/multi_object.3mf", "-o"])
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());
}

#[test]
fn test_3mf_with_all_view_presets() {
    let dir = tempfile::tempdir().unwrap();

    for view in ["iso", "front", "top", "print", "print-front", "print-grid"] {
        let output = dir.path().join(format!("{}.png", view));

        let status = Command::new(env!("CARGO_BIN_EXE_stl-render"))
            .args(["fixtures/cube.3mf", "-o"])
            .arg(&output)
            .args(["--view", view])
            .status()
            .unwrap();

        assert!(status.success(), "view {} failed", view);
        assert!(output.exists(), "view {} produced no output", view);
    }
}

#[test]
fn test_3mf_stl_produce_same_output() {
    use stl_render::{RenderConfigBuilder, ViewPreset, render_to_image};

    let stl_config = RenderConfigBuilder::new("fixtures/cube.stl", "-")
        .view(ViewPreset::Iso)
        .size(128)
        .build();

    let tmf3_config = RenderConfigBuilder::new("fixtures/cube.3mf", "-")
        .view(ViewPreset::Iso)
        .size(128)
        .build();

    let (stl_image, stl_meta) = render_to_image(&stl_config).unwrap();
    let (tmf3_image, tmf3_meta) = render_to_image(&tmf3_config).unwrap();

    assert_eq!(stl_meta.triangle_count, tmf3_meta.triangle_count);
    assert_eq!(stl_image.width(), tmf3_image.width());
    assert_eq!(stl_image.height(), tmf3_image.height());
}

#[test]
fn test_3mf_format_autodetected() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("out.png");

    // Copy 3mf to a file with .stl extension - should still work due to content detection
    let misnamed = dir.path().join("cube.stl");
    std::fs::copy("fixtures/cube.3mf", &misnamed).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_stl-render"))
        .arg(&misnamed)
        .args(["-o"])
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());
}

#[test]
fn test_3mf_in_batch_mode() {
    let dir = tempfile::tempdir().unwrap();
    let output_dir = dir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_stl-render"))
        .args(["fixtures/cube.3mf", "fixtures/sphere.3mf", "-o"])
        .arg(&output_dir)
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output_dir.join("cube.png").exists());
    assert!(output_dir.join("sphere.png").exists());
}

#[test]
fn test_malformed_3mf_error() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("out.png");

    let output_result = Command::new(env!("CARGO_BIN_EXE_stl-render"))
        .args(["fixtures/malformed.3mf", "-o"])
        .arg(&output)
        .output()
        .unwrap();

    assert!(!output_result.status.success());
    let stderr = String::from_utf8_lossy(&output_result.stderr);
    assert!(
        stderr.contains("invalid") || stderr.contains("ZIP") || stderr.contains("error"),
        "expected error message about invalid 3MF, got: {}",
        stderr
    );
}

#[test]
fn test_missing_model_3mf_error() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("out.png");

    let output_result = Command::new(env!("CARGO_BIN_EXE_stl-render"))
        .args(["fixtures/missing_model.3mf", "-o"])
        .arg(&output)
        .output()
        .unwrap();

    assert!(!output_result.status.success());
    let stderr = String::from_utf8_lossy(&output_result.stderr);
    assert!(
        stderr.contains("model") || stderr.contains("3dmodel") || stderr.contains("missing"),
        "expected error about missing model file, got: {}",
        stderr
    );
}

// =============================================================================
// OBJ Format Tests
// =============================================================================

#[test]
fn test_render_obj_cube() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("cube.png");

    let status = Command::new(env!("CARGO_BIN_EXE_stl-render"))
        .args(["fixtures/cube.obj", "-o"])
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());

    let img = image::open(&output).unwrap();
    assert_eq!(img.width(), 512);
    assert_eq!(img.height(), 512);
}

#[test]
fn test_render_obj_sphere() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("sphere.png");

    let status = Command::new(env!("CARGO_BIN_EXE_stl-render"))
        .args(["fixtures/sphere.obj", "-o"])
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());
}

#[test]
fn test_obj_with_all_view_presets() {
    let dir = tempfile::tempdir().unwrap();

    for view in ["iso", "front", "top", "print", "print-front", "print-grid"] {
        let output = dir.path().join(format!("{}.png", view));

        let status = Command::new(env!("CARGO_BIN_EXE_stl-render"))
            .args(["fixtures/cube.obj", "-o"])
            .arg(&output)
            .args(["--view", view])
            .status()
            .unwrap();

        assert!(status.success(), "view {} failed", view);
        assert!(output.exists(), "view {} produced no output", view);
    }
}

#[test]
fn test_obj_stl_produce_same_triangle_count() {
    use stl_render::{RenderConfigBuilder, ViewPreset, render_to_image};

    let stl_config = RenderConfigBuilder::new("fixtures/cube.stl", "-")
        .view(ViewPreset::Iso)
        .size(128)
        .build();

    let obj_config = RenderConfigBuilder::new("fixtures/cube.obj", "-")
        .view(ViewPreset::Iso)
        .size(128)
        .build();

    let (stl_image, stl_meta) = render_to_image(&stl_config).unwrap();
    let (obj_image, obj_meta) = render_to_image(&obj_config).unwrap();

    assert_eq!(stl_meta.triangle_count, obj_meta.triangle_count);
    assert_eq!(stl_image.width(), obj_image.width());
    assert_eq!(stl_image.height(), obj_image.height());
}

#[test]
fn test_obj_format_autodetected() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("out.png");

    // Copy obj to a file with .stl extension - should still work due to content detection
    let misnamed = dir.path().join("cube.stl");
    std::fs::copy("fixtures/cube.obj", &misnamed).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_stl-render"))
        .arg(&misnamed)
        .args(["-o"])
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());
}

#[test]
fn test_obj_in_batch_mode() {
    let dir = tempfile::tempdir().unwrap();
    let output_dir = dir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_stl-render"))
        .args(["fixtures/cube.obj", "fixtures/sphere.obj", "-o"])
        .arg(&output_dir)
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output_dir.join("cube.png").exists());
    assert!(output_dir.join("sphere.png").exists());
}

#[test]
fn test_obj_with_animation() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("cube.gif");

    let status = Command::new(env!("CARGO_BIN_EXE_stl-render"))
        .args(["fixtures/cube.obj", "-o"])
        .arg(&output)
        .args(["--animate", "--frames", "4"])
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());

    let data = std::fs::read(&output).unwrap();
    assert!(data.starts_with(b"GIF89a") || data.starts_with(b"GIF87a"));
}

// =============================================================================
// Animated GIF Tests
// =============================================================================

#[test]
fn test_animate_flag_produces_gif() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("cube.gif");

    let status = Command::new(env!("CARGO_BIN_EXE_stl-render"))
        .args(["fixtures/cube.stl", "-o"])
        .arg(&output)
        .arg("--animate")
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());

    // Verify it's a valid GIF by checking magic bytes
    let data = std::fs::read(&output).unwrap();
    assert!(data.starts_with(b"GIF89a") || data.starts_with(b"GIF87a"));
}

#[test]
fn test_animate_custom_frames() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("cube.gif");

    let status = Command::new(env!("CARGO_BIN_EXE_stl-render"))
        .args(["fixtures/cube.stl", "-o"])
        .arg(&output)
        .args(["--animate", "--frames", "8"])
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());
}

#[test]
fn test_animate_custom_frame_delay() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("cube.gif");

    let status = Command::new(env!("CARGO_BIN_EXE_stl-render"))
        .args(["fixtures/cube.stl", "-o"])
        .arg(&output)
        .args(["--animate", "--frame-delay", "200"])
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());
}

#[test]
fn test_animate_with_material_color() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("cube.gif");

    let status = Command::new(env!("CARGO_BIN_EXE_stl-render"))
        .args(["fixtures/cube.stl", "-o"])
        .arg(&output)
        .args(["--animate", "--material-color", "tan"])
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());
}

#[test]
fn test_animate_with_3mf() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("cube.gif");

    let status = Command::new(env!("CARGO_BIN_EXE_stl-render"))
        .args(["fixtures/cube.3mf", "-o"])
        .arg(&output)
        .arg("--animate")
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());
}

#[test]
fn test_animate_verbose_shows_frame_progress() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("cube.gif");

    let result = Command::new(env!("CARGO_BIN_EXE_stl-render"))
        .args(["fixtures/cube.stl", "-o"])
        .arg(&output)
        .args(["--animate", "--frames", "4", "--verbose"])
        .output()
        .unwrap();

    assert!(result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("frame 1/4"), "should show frame progress");
    assert!(stderr.contains("frame 4/4"), "should show final frame");
}

#[test]
fn test_animate_without_flag_produces_png() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("cube.gif"); // .gif extension but no --animate flag

    let status = Command::new(env!("CARGO_BIN_EXE_stl-render"))
        .args(["fixtures/cube.stl", "-o"])
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());

    // Without --animate flag, should produce PNG even with .gif extension
    let data = std::fs::read(&output).unwrap();
    assert!(
        data.starts_with(&[0x89, 0x50, 0x4E, 0x47]), // PNG magic
        "without --animate flag should produce PNG"
    );
}

#[test]
fn test_animate_builder_api() {
    use stl_render::{RenderConfigBuilder, render};

    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("cube.gif");

    let config = RenderConfigBuilder::new("fixtures/cube.stl", &output)
        .animate()
        .frames(8)
        .frame_delay(50)
        .build();

    let result = render(&config);
    assert!(result.is_ok());
    assert!(output.exists());

    let data = std::fs::read(&output).unwrap();
    assert!(data.starts_with(b"GIF89a") || data.starts_with(b"GIF87a"));
}
