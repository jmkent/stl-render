#!/usr/bin/env python3
"""Generate STL, OBJ, and 3MF test fixtures for stl-render.

Creates basic geometric shapes as binary STL, ASCII STL, OBJ, and 3MF files.
"""

import argparse
import io
import math
import zipfile
from pathlib import Path
from xml.etree import ElementTree as ET

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


def save_obj(m: mesh.Mesh, path: Path):
    """Save mesh to OBJ file.

    OBJ format:
    - v x y z  (vertices)
    - f v1 v2 v3  (faces, 1-indexed)
    """
    # Collect unique vertices and build index map
    vertices = []
    vertex_map = {}
    faces = []

    for tri_verts in m.vectors:
        face_indices = []
        for vert in tri_verts:
            # Round to avoid floating point duplicates
            key = tuple(round(float(v), 6) for v in vert)
            if key not in vertex_map:
                vertex_map[key] = len(vertices) + 1  # OBJ is 1-indexed
                vertices.append(vert)
            face_indices.append(vertex_map[key])
        faces.append(face_indices)

    with open(path, 'w') as f:
        f.write(f"# OBJ file generated by stl-render fixture generator\n")
        f.write(f"# Vertices: {len(vertices)}, Faces: {len(faces)}\n\n")

        # Write vertices
        for v in vertices:
            f.write(f"v {v[0]:.6f} {v[1]:.6f} {v[2]:.6f}\n")

        f.write("\n")

        # Write faces
        for face in faces:
            f.write(f"f {face[0]} {face[1]} {face[2]}\n")

    print(f"  {path.name}: {len(faces)} triangles")


def mesh_to_3mf_model_xml(meshes: list[tuple[str, mesh.Mesh]]) -> bytes:
    """Convert mesh(es) to 3MF model XML content.

    Args:
        meshes: List of (name, mesh) tuples. Each becomes a separate object.

    Returns:
        UTF-8 encoded XML bytes.
    """
    NS = "http://schemas.microsoft.com/3dmanufacturing/core/2015/02"

    # Create root element with namespace
    model = ET.Element("model")
    model.set("unit", "millimeter")
    model.set("xmlns", NS)

    resources = ET.SubElement(model, "resources")
    build = ET.SubElement(model, "build")

    for obj_id, (name, m) in enumerate(meshes, start=1):
        # Collect unique vertices and build index map
        vertices = []
        vertex_map = {}
        triangles = []

        for tri_verts in m.vectors:
            tri_indices = []
            for v in tri_verts:
                key = (round(v[0], 6), round(v[1], 6), round(v[2], 6))
                if key not in vertex_map:
                    vertex_map[key] = len(vertices)
                    vertices.append(key)
                tri_indices.append(vertex_map[key])
            triangles.append(tuple(tri_indices))

        # Create object element
        obj = ET.SubElement(resources, "object")
        obj.set("id", str(obj_id))
        obj.set("type", "model")
        if name:
            obj.set("name", name)

        mesh_elem = ET.SubElement(obj, "mesh")

        # Vertices
        verts_elem = ET.SubElement(mesh_elem, "vertices")
        for x, y, z in vertices:
            v = ET.SubElement(verts_elem, "vertex")
            v.set("x", f"{x:.6g}")
            v.set("y", f"{y:.6g}")
            v.set("z", f"{z:.6g}")

        # Triangles
        tris_elem = ET.SubElement(mesh_elem, "triangles")
        for v1, v2, v3 in triangles:
            t = ET.SubElement(tris_elem, "triangle")
            t.set("v1", str(v1))
            t.set("v2", str(v2))
            t.set("v3", str(v3))

        # Add to build
        item = ET.SubElement(build, "item")
        item.set("objectid", str(obj_id))

    # Generate XML with declaration
    xml_decl = b'<?xml version="1.0" encoding="UTF-8"?>\n'
    tree = ET.ElementTree(model)
    buf = io.BytesIO()
    tree.write(buf, encoding="UTF-8", xml_declaration=False)
    return xml_decl + buf.getvalue()


def save_3mf(meshes: list[tuple[str, mesh.Mesh]], path: Path):
    """Save mesh(es) to 3MF file.

    Args:
        meshes: List of (name, mesh) tuples. Use [("", mesh)] for single object.
        path: Output file path.
    """
    # Content types XML
    content_types = b'''<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/>
</Types>'''

    # Relationships XML
    rels = b'''<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/>
</Relationships>'''

    # Model XML
    model_xml = mesh_to_3mf_model_xml(meshes)

    # Create ZIP archive
    with zipfile.ZipFile(path, 'w', zipfile.ZIP_DEFLATED) as zf:
        zf.writestr("[Content_Types].xml", content_types)
        zf.writestr("_rels/.rels", rels)
        zf.writestr("3D/3dmodel.model", model_xml)

    total_triangles = sum(len(m.vectors) for _, m in meshes)
    obj_count = len(meshes)
    if obj_count == 1:
        print(f"  {path.name}: {total_triangles} triangles")
    else:
        print(f"  {path.name}: {total_triangles} triangles ({obj_count} objects)")


def colored_model_xml(name, vertices, triangles, palette):
    """Build a 3MF model XML with a material-extension colorgroup.

    Args:
        name: Object name.
        vertices: List of (x, y, z) tuples.
        triangles: List of (v1, v2, v3, p1, p2, p3) tuples. If p1/p2/p3 are
            None the triangle is emitted without a pid (uncolored, falls back
            to the renderer's material color).
        palette: List of "#RRGGBB" color strings for colorgroup id 1.

    Returns:
        UTF-8 encoded XML bytes.
    """
    CORE = "http://schemas.microsoft.com/3dmanufacturing/core/2015/02"
    MAT = "http://schemas.microsoft.com/3dmanufacturing/material/2015/02"

    lines = ['<?xml version="1.0" encoding="UTF-8"?>']
    lines.append(f'<model unit="millimeter" xmlns="{CORE}" xmlns:m="{MAT}">')
    lines.append('  <resources>')
    lines.append('    <m:colorgroup id="1">')
    for color in palette:
        lines.append(f'      <m:color color="{color}"/>')
    lines.append('    </m:colorgroup>')
    lines.append(f'    <object id="2" type="model" name="{name}">')
    lines.append('      <mesh>')
    lines.append('        <vertices>')
    for x, y, z in vertices:
        lines.append(f'          <vertex x="{x:.6g}" y="{y:.6g}" z="{z:.6g}"/>')
    lines.append('        </vertices>')
    lines.append('        <triangles>')
    for v1, v2, v3, p1, p2, p3 in triangles:
        if p1 is None:
            lines.append(f'          <triangle v1="{v1}" v2="{v2}" v3="{v3}"/>')
        else:
            lines.append(
                f'          <triangle v1="{v1}" v2="{v2}" v3="{v3}" '
                f'pid="1" p1="{p1}" p2="{p2}" p3="{p3}"/>'
            )
    lines.append('        </triangles>')
    lines.append('      </mesh>')
    lines.append('    </object>')
    lines.append('  </resources>')
    lines.append('  <build>')
    lines.append('    <item objectid="2"/>')
    lines.append('  </build>')
    lines.append('</model>')
    return ("\n".join(lines) + "\n").encode("utf-8")


def save_colored_3mf(model_xml: bytes, path: Path, description: str):
    """Write a colored 3MF (model XML supplied directly) into an OPC package."""
    content_types = b'''<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/>
</Types>'''

    rels = b'''<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/>
</Relationships>'''

    with zipfile.ZipFile(path, 'w', zipfile.ZIP_DEFLATED) as zf:
        zf.writestr("[Content_Types].xml", content_types)
        zf.writestr("_rels/.rels", rels)
        zf.writestr("3D/3dmodel.model", model_xml)

    print(f"  {path.name}: {description}")


# Unit cube corner vertices, shared by the colored cube fixtures.
_CUBE_VERTS = [
    (-0.5, -0.5, -0.5), (0.5, -0.5, -0.5), (0.5, 0.5, -0.5), (-0.5, 0.5, -0.5),
    (-0.5, -0.5, 0.5), (0.5, -0.5, 0.5), (0.5, 0.5, 0.5), (-0.5, 0.5, 0.5),
]


def colored_cube_model_xml():
    """Cube with a distinct flat color on each of the six faces."""
    # (v1, v2, v3, face_color_index) for each triangle, two per face.
    faces = [
        (0, 3, 1, 0), (1, 3, 2, 0),  # bottom  -> red
        (4, 5, 7, 1), (5, 6, 7, 1),  # top     -> green
        (0, 1, 4, 2), (1, 5, 4, 2),  # front   -> blue
        (2, 3, 6, 3), (3, 7, 6, 3),  # back    -> yellow
        (0, 4, 3, 4), (3, 4, 7, 4),  # left    -> magenta
        (1, 2, 5, 5), (2, 6, 5, 5),  # right   -> cyan
    ]
    triangles = [(v1, v2, v3, c, c, c) for v1, v2, v3, c in faces]
    palette = ["#FF0000", "#00FF00", "#0000FF", "#FFFF00", "#FF00FF", "#00FFFF"]
    return colored_model_xml("colored_cube", _CUBE_VERTS, triangles, palette)


def gradient_model_xml():
    """Flat quad with a different palette color at each corner (per-vertex)."""
    vertices = [
        (-1.0, -1.0, 0.0),  # 0 -> red
        (1.0, -1.0, 0.0),   # 1 -> green
        (1.0, 1.0, 0.0),    # 2 -> blue
        (-1.0, 1.0, 0.0),   # 3 -> white
    ]
    triangles = [
        (0, 1, 2, 0, 1, 2),
        (0, 2, 3, 0, 2, 3),
    ]
    palette = ["#FF0000", "#00FF00", "#0000FF", "#FFFFFF"]
    return colored_model_xml("gradient", vertices, triangles, palette)


def partial_colors_model_xml():
    """Cube where only some faces carry colors; the rest are uncolored."""
    # Bottom red, top green; remaining faces have no pid (material color).
    faces = [
        (0, 3, 1, 0), (1, 3, 2, 0),  # bottom -> red
        (4, 5, 7, 1), (5, 6, 7, 1),  # top    -> green
        (0, 1, 4, None), (1, 5, 4, None),  # front  -> uncolored
        (2, 3, 6, None), (3, 7, 6, None),  # back   -> uncolored
        (0, 4, 3, None), (3, 4, 7, None),  # left   -> uncolored
        (1, 2, 5, None), (2, 6, 5, None),  # right  -> uncolored
    ]
    triangles = [
        (v1, v2, v3, c, c, c) if c is not None else (v1, v2, v3, None, None, None)
        for v1, v2, v3, c in faces
    ]
    palette = ["#FF0000", "#00FF00"]
    return colored_model_xml("partial_colors", _CUBE_VERTS, triangles, palette)


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

    # OBJ files
    print("\nOBJ files:")
    obj_shapes = [
        ("cube.obj", create_cube()),
        ("sphere.obj", create_sphere(subdivisions=args.sphere_subdivisions)),
        ("single_triangle.obj", create_single_triangle()),
    ]

    for name, m in obj_shapes:
        save_obj(m, output_dir / name)

    # 3MF files
    print("\n3MF files:")
    tmf3_shapes = [
        ("cube.3mf", [("cube", create_cube())]),
        ("sphere.3mf", [("sphere", create_sphere(subdivisions=args.sphere_subdivisions))]),
        ("single_triangle.3mf", [("triangle", create_single_triangle())]),
    ]

    for name, meshes in tmf3_shapes:
        save_3mf(meshes, output_dir / name)

    # Multi-object 3MF (multiple objects in one file)
    print("\n3MF multi-object files:")
    multi_objects = [
        ("cube", create_cube()),
        ("small_sphere", create_sphere(radius=0.3, subdivisions=2)),
    ]
    # Offset the sphere so they don't overlap
    sphere_mesh = multi_objects[1][1]
    for i in range(len(sphere_mesh.vectors)):
        sphere_mesh.vectors[i] += np.array([1.5, 0, 0])
    save_3mf(multi_objects, output_dir / "multi_object.3mf")

    # Colored 3MF files (material extension colorgroups)
    print("\n3MF colored files:")
    save_colored_3mf(
        colored_cube_model_xml(),
        output_dir / "colored_cube.3mf",
        "12 triangles, 6 per-face colors",
    )
    save_colored_3mf(
        gradient_model_xml(),
        output_dir / "gradient.3mf",
        "2 triangles, 4 per-vertex colors (gradient)",
    )
    save_colored_3mf(
        partial_colors_model_xml(),
        output_dir / "partial_colors.3mf",
        "12 triangles, 2 colored faces + 4 uncolored",
    )

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

    # Create malformed 3MF (not a valid ZIP)
    malformed_3mf_path = output_dir / "malformed.3mf"
    with open(malformed_3mf_path, "wb") as f:
        f.write(b"This is not a ZIP file")
    print(f"  malformed.3mf: invalid (not a ZIP)")

    # Create 3MF with missing model file
    missing_model_3mf_path = output_dir / "missing_model.3mf"
    with zipfile.ZipFile(missing_model_3mf_path, 'w') as zf:
        zf.writestr("[Content_Types].xml", b"<Types/>")
        # Intentionally missing 3D/3dmodel.model
    print(f"  missing_model.3mf: invalid (missing model file)")

    total_files = len(shapes) + len(ascii_shapes) + len(obj_shapes) + len(tmf3_shapes) + 1 + 4  # +1 for multi_object, +4 for special files

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
