#!/usr/bin/env python3
"""Generate STL test fixtures for stl-render.

Creates basic geometric shapes as both binary and ASCII STL files.
"""

import argparse
import math
from pathlib import Path

import numpy as np
import stl
from stl import mesh


def create_cube(size: float = 1.0) -> mesh.Mesh:
    """Create a unit cube centered at origin."""
    s = size / 2
    vertices = np.array([
        [-s, -s, -s], [+s, -s, -s], [+s, +s, -s], [-s, +s, -s],  # bottom
        [-s, -s, +s], [+s, -s, +s], [+s, +s, +s], [-s, +s, +s],  # top
    ])

    # 12 triangles (2 per face)
    faces = np.array([
        [0, 3, 1], [1, 3, 2],  # bottom
        [4, 5, 7], [5, 6, 7],  # top
        [0, 1, 4], [1, 5, 4],  # front
        [2, 3, 6], [3, 7, 6],  # back
        [0, 4, 3], [3, 4, 7],  # left
        [1, 2, 5], [2, 6, 5],  # right
    ])

    cube = mesh.Mesh(np.zeros(faces.shape[0], dtype=mesh.Mesh.dtype))
    for i, f in enumerate(faces):
        for j in range(3):
            cube.vectors[i][j] = vertices[f[j]]

    return cube


def create_sphere(radius: float = 0.5, subdivisions: int = 3) -> mesh.Mesh:
    """Create an icosphere by subdivision."""
    # Golden ratio
    phi = (1 + math.sqrt(5)) / 2

    # Icosahedron vertices
    vertices = [
        [-1, phi, 0], [1, phi, 0], [-1, -phi, 0], [1, -phi, 0],
        [0, -1, phi], [0, 1, phi], [0, -1, -phi], [0, 1, -phi],
        [phi, 0, -1], [phi, 0, 1], [-phi, 0, -1], [-phi, 0, 1],
    ]
    vertices = [np.array(v) / np.linalg.norm(v) * radius for v in vertices]

    # Icosahedron faces
    faces = [
        [0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11],
        [1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8],
        [3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9],
        [4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1],
    ]

    # Subdivide
    for _ in range(subdivisions):
        new_faces = []
        midpoint_cache = {}

        def get_midpoint(i1, i2):
            key = tuple(sorted([i1, i2]))
            if key in midpoint_cache:
                return midpoint_cache[key]
            p1, p2 = vertices[i1], vertices[i2]
            mid = (p1 + p2) / 2
            mid = mid / np.linalg.norm(mid) * radius
            idx = len(vertices)
            vertices.append(mid)
            midpoint_cache[key] = idx
            return idx

        for f in faces:
            a, b, c = get_midpoint(f[0], f[1]), get_midpoint(f[1], f[2]), get_midpoint(f[2], f[0])
            new_faces.extend([
                [f[0], a, c], [f[1], b, a], [f[2], c, b], [a, b, c]
            ])
        faces = new_faces

    sphere = mesh.Mesh(np.zeros(len(faces), dtype=mesh.Mesh.dtype))
    for i, f in enumerate(faces):
        for j in range(3):
            sphere.vectors[i][j] = vertices[f[j]]

    return sphere


def create_cylinder(radius: float = 0.5, height: float = 1.0, segments: int = 32) -> mesh.Mesh:
    """Create a cylinder centered at origin, aligned with Z axis."""
    h = height / 2
    faces_list = []

    for i in range(segments):
        angle1 = 2 * math.pi * i / segments
        angle2 = 2 * math.pi * (i + 1) / segments

        x1, y1 = radius * math.cos(angle1), radius * math.sin(angle1)
        x2, y2 = radius * math.cos(angle2), radius * math.sin(angle2)

        # Side faces (2 triangles per segment)
        faces_list.append([[x1, y1, -h], [x2, y2, -h], [x1, y1, h]])
        faces_list.append([[x2, y2, -h], [x2, y2, h], [x1, y1, h]])

        # Top cap
        faces_list.append([[0, 0, h], [x1, y1, h], [x2, y2, h]])

        # Bottom cap
        faces_list.append([[0, 0, -h], [x2, y2, -h], [x1, y1, -h]])

    cyl = mesh.Mesh(np.zeros(len(faces_list), dtype=mesh.Mesh.dtype))
    for i, f in enumerate(faces_list):
        cyl.vectors[i] = np.array(f)

    return cyl


def create_tall_column(width: float = 0.2, height: float = 2.0) -> mesh.Mesh:
    """Create a tall thin column (tests aspect ratio handling)."""
    return create_box(width, width, height)


def create_flat_tile(width: float = 2.0, height: float = 0.1) -> mesh.Mesh:
    """Create a flat tile (tests near-planar geometry)."""
    return create_box(width, width, height)


def create_long_beam(length: float = 3.0, size: float = 0.2) -> mesh.Mesh:
    """Create a long beam (tests elongated geometry)."""
    return create_box(length, size, size)


def create_box(width: float, depth: float, height: float) -> mesh.Mesh:
    """Create a box with arbitrary dimensions, centered at origin."""
    w, d, h = width / 2, depth / 2, height / 2
    vertices = np.array([
        [-w, -d, -h], [+w, -d, -h], [+w, +d, -h], [-w, +d, -h],
        [-w, -d, +h], [+w, -d, +h], [+w, +d, +h], [-w, +d, +h],
    ])

    faces = np.array([
        [0, 3, 1], [1, 3, 2],  # bottom
        [4, 5, 7], [5, 6, 7],  # top
        [0, 1, 4], [1, 5, 4],  # front
        [2, 3, 6], [3, 7, 6],  # back
        [0, 4, 3], [3, 4, 7],  # left
        [1, 2, 5], [2, 6, 5],  # right
    ])

    box = mesh.Mesh(np.zeros(faces.shape[0], dtype=mesh.Mesh.dtype))
    for i, f in enumerate(faces):
        for j in range(3):
            box.vectors[i][j] = vertices[f[j]]

    return box


def create_single_triangle() -> mesh.Mesh:
    """Create a single triangle (minimal valid STL)."""
    tri = mesh.Mesh(np.zeros(1, dtype=mesh.Mesh.dtype))
    tri.vectors[0] = np.array([
        [0, 0, 0],
        [1, 0, 0],
        [0.5, 1, 0],
    ])
    return tri


def create_degenerate_triangles() -> mesh.Mesh:
    """Create mesh with degenerate (zero-area) triangles."""
    m = mesh.Mesh(np.zeros(3, dtype=mesh.Mesh.dtype))

    # Normal triangle
    m.vectors[0] = np.array([[0, 0, 0], [1, 0, 0], [0.5, 1, 0]])

    # Degenerate: all points collinear
    m.vectors[1] = np.array([[0, 0, 1], [0.5, 0, 1], [1, 0, 1]])

    # Degenerate: all points identical
    m.vectors[2] = np.array([[0, 0, 2], [0, 0, 2], [0, 0, 2]])

    return m


def save_stl(m: mesh.Mesh, path: Path, ascii_format: bool = False):
    """Save mesh to STL file."""
    if ascii_format:
        m.save(str(path), mode=stl.Mode.ASCII)
    else:
        m.save(str(path), mode=stl.Mode.BINARY)
    print(f"  {path.name}: {len(m.vectors)} triangles")


def create_large_sphere(target_triangles: int) -> mesh.Mesh:
    """Create a sphere with approximately target_triangles triangles.

    Subdivisions: 0=20, 1=80, 2=320, 3=1280, 4=5120, 5=20480, 6=81920, 7=327680, 8=1310720
    """
    import math as m
    subdivisions = max(0, int(m.log(target_triangles / 20, 4)))
    return create_sphere(subdivisions=subdivisions)


def main():
    parser = argparse.ArgumentParser(description="Generate STL test fixtures")
    parser.add_argument(
        "-o", "--output",
        type=Path,
        default=Path("fixtures"),
        help="Output directory (default: fixtures)",
    )
    parser.add_argument(
        "--sphere-subdivisions",
        type=int,
        default=3,
        help="Sphere subdivision level (default: 3, produces 1280 triangles)",
    )
    parser.add_argument(
        "--large",
        action="store_true",
        help="Also generate large test files (~50MB and ~500MB, not for committing)",
    )
    args = parser.parse_args()

    output_dir = args.output
    output_dir.mkdir(parents=True, exist_ok=True)

    print(f"Generating fixtures in {output_dir}/\n")

    # Basic shapes - binary format
    print("Binary STL files:")
    shapes = [
        ("cube.stl", create_cube()),
        ("sphere.stl", create_sphere(subdivisions=args.sphere_subdivisions)),
        ("cylinder.stl", create_cylinder()),
        ("tall_column.stl", create_tall_column()),
        ("flat_tile.stl", create_flat_tile()),
        ("long_beam.stl", create_long_beam()),
        ("single_triangle.stl", create_single_triangle()),
        ("degenerate.stl", create_degenerate_triangles()),
    ]

    for name, m in shapes:
        save_stl(m, output_dir / name, ascii_format=False)

    # ASCII versions of key shapes
    print("\nASCII STL files:")
    ascii_shapes = [
        ("cube_ascii.stl", create_cube()),
        ("single_triangle_ascii.stl", create_single_triangle()),
    ]

    for name, m in ascii_shapes:
        save_stl(m, output_dir / name, ascii_format=True)

    # Create an empty STL (just header, 0 triangles) - manually
    empty_path = output_dir / "empty.stl"
    with open(empty_path, "wb") as f:
        f.write(b"\x00" * 80)  # header
        f.write((0).to_bytes(4, "little"))  # triangle count
    print(f"\nSpecial files:")
    print(f"  empty.stl: 0 triangles (valid but empty)")

    # Create truncated STL (invalid)
    truncated_path = output_dir / "truncated.stl"
    with open(truncated_path, "wb") as f:
        f.write(b"\x00" * 80)  # header
        f.write((10).to_bytes(4, "little"))  # claims 10 triangles
        f.write(b"\x00" * 25)  # but only half a triangle of data
    print(f"  truncated.stl: invalid (truncated data)")

    total_files = len(shapes) + len(ascii_shapes) + 2

    # Generate large files for performance testing (optional)
    if args.large:
        print("\nLarge STL files (for performance testing):")
        print("  Generating large_1m.stl (~1M triangles, ~50MB)...")
        large_1m = create_sphere(subdivisions=8)  # 1,310,720 triangles
        save_stl(large_1m, output_dir / "large_1m.stl", ascii_format=False)

        print("  Generating large_10m.stl (~10M triangles, ~500MB)...")
        # Create multiple high-detail spheres at different positions
        # 8 spheres × 1.3M triangles ≈ 10.5M triangles
        large_10m = mesh.Mesh(np.zeros(1310720 * 8, dtype=mesh.Mesh.dtype))
        offsets = [
            (-2, -2, -2), (2, -2, -2), (-2, 2, -2), (2, 2, -2),
            (-2, -2, 2), (2, -2, 2), (-2, 2, 2), (2, 2, 2),
        ]
        for i, (ox, oy, oz) in enumerate(offsets):
            sphere = create_sphere(subdivisions=8)
            for j in range(len(sphere.vectors)):
                large_10m.vectors[i * 1310720 + j] = sphere.vectors[j] + np.array([ox, oy, oz])
        save_stl(large_10m, output_dir / "large_10m.stl", ascii_format=False)
        total_files += 2

    print(f"\nDone. Generated {total_files} fixture files.")


if __name__ == "__main__":
    main()
