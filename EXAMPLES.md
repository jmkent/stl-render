# stl-render Examples

This document showcases various rendering options available in stl-render.

## Quick Start

```bash
stl-render model.stl -o preview.png
```

## 3DBenchy

[3DBenchy](https://www.3dbenchy.com) is a standard public domain 3D printing benchmark model. These examples demonstrate stl-render's capabilities with a real-world print model (225K triangles).

| Blue Grey | Tan |
|-----------|-----|
| ![Benchy Blue Grey](examples/benchy_print_bluegrey.png) | ![Benchy Tan](examples/benchy_print_tan.png) |

```bash
stl-render 3DBenchy.stl -o benchy.png --view print --material-color "#708090" --aa 4x
```

## View Presets

### Print Bed View (`--view print`)

The `print` view is designed for 3D printing previews. It uses Z-up orientation so the model appears as it would on a print bed, with a slight tilt to show the top surface.

| Cube | Sphere | Cylinder |
|------|--------|----------|
| ![Cube Print](examples/hero_cube.png) | ![Sphere Print](examples/hero_sphere.png) | ![Cylinder Print](examples/hero_cylinder.png) |

```bash
stl-render model.stl -o preview.png --view print --material-color "#C19A6B"
```

### View Comparison

| Front | Top | Isometric | Print |
|-------|-----|-----------|-------|
| ![Front](examples/view_front.png) | ![Top](examples/view_top.png) | ![Iso](examples/view_iso.png) | ![Print](examples/view_print.png) |

```bash
stl-render model.stl -o preview.png --view front
stl-render model.stl -o preview.png --view top
stl-render model.stl -o preview.png --view iso
stl-render model.stl -o preview.png --view print
```

Available presets: `front`, `back`, `left`, `right`, `top`, `bottom`, `iso`, `print`

## Material Colors

Use `--material-color` with hex colors to match common filament colors:

| Blue Grey (`#708090`) | Tan (`#C19A6B`) |
|-----------------------|-----------------|
| ![Blue Grey Sphere](examples/sphere_print_bluegrey.png) | ![Tan Sphere](examples/sphere_iso_tan.png) |

```bash
stl-render model.stl -o preview.png --material-color "#708090"  # Blue grey
stl-render model.stl -o preview.png --material-color "#C19A6B"  # Tan
```

Other common filament colors:
- White: `#FFFFFF`
- Black: `#1A1A1A`
- Red: `#CC3333`
- Orange: `#FF6600`
- Green: `#339933`
- Blue: `#3366CC`

## Lighting Presets

| Flat | Studio (default) | Technical |
|------|------------------|-----------|
| ![Flat](examples/lighting_flat.png) | ![Studio](examples/lighting_studio.png) | ![Technical](examples/lighting_technical.png) |

```bash
stl-render model.stl -o preview.png --lighting flat       # Single front light
stl-render model.stl -o preview.png --lighting studio     # Key + fill + rim (default)
stl-render model.stl -o preview.png --lighting technical  # Uniform multi-directional
```

- **Flat**: Single front-facing light. Good for technical drawings.
- **Studio**: Three-point lighting (key, fill, rim). Good for presentation renders.
- **Technical**: Even illumination from multiple directions. Good for inspection.

## Background Options

| Transparent (default) | Solid White | Solid Dark |
|-----------------------|-------------|------------|
| ![Transparent](examples/bg_transparent.png) | ![White](examples/bg_solid_white.png) | ![Dark](examples/bg_solid_dark.png) |

```bash
stl-render model.stl -o preview.png --background transparent
stl-render model.stl -o preview.png --background solid --background-color "#ffffff"
stl-render model.stl -o preview.png --background solid --background-color "#2d2d2d"
```

## Anti-Aliasing

Higher AA levels render at increased resolution then downsample for smoother edges.

| None | 2x (default) | 4x |
|------|--------------|-----|
| ![None](examples/aa_none.png) | ![2x](examples/aa_2x.png) | ![4x](examples/aa_4x.png) |

```bash
stl-render model.stl -o preview.png --aa none  # Fastest, aliased edges
stl-render model.stl -o preview.png --aa 2x    # Good quality (default)
stl-render model.stl -o preview.png --aa 4x    # Best quality, 4x render time
```

## Custom Camera Angles

For precise control, use `--azimuth` and `--elevation` instead of presets:

```bash
# Azimuth: rotation around vertical axis (0-360)
# Elevation: angle above horizon (-90 to 90)

stl-render model.stl -o preview.png --azimuth 45 --elevation 30
stl-render model.stl -o preview.png --azimuth 135 --elevation 15
```

## Image Size and Padding

```bash
# Custom dimensions
stl-render model.stl -o preview.png --width 1024 --height 768

# Adjust padding (space around model)
stl-render model.stl -o preview.png --padding 0.0   # No margin
stl-render model.stl -o preview.png --padding 0.2   # 20% margin
```

## Metadata Output

Export render information as JSON:

```bash
stl-render model.stl -o preview.png --metadata info.json
```

```json
{
  "input_file": "model.stl",
  "triangle_count": 12,
  "bounding_box": {
    "min": [-0.5, -0.5, -0.5],
    "max": [0.5, 0.5, 0.5]
  },
  "dimensions": [1.0, 1.0, 1.0]
}
```

## Piping

```bash
# Read from stdin
cat model.stl | stl-render - -o preview.png

# Write to stdout
stl-render model.stl -o - > preview.png

# Full pipeline
cat model.stl | stl-render - -o - | convert - thumbnail.jpg
```

## Combining Options

```bash
# High-quality print preview with custom color
stl-render model.stl -o preview.png \
    --view print \
    --material-color "#708090" \
    --lighting studio \
    --aa 4x \
    --width 1024 \
    --height 1024

# Technical documentation render
stl-render model.stl -o preview.png \
    --view front \
    --material-color "#cccccc" \
    --lighting technical \
    --background solid \
    --background-color "#ffffff"
```
