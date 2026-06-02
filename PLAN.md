# STL Renderer Project Plan

This plan tracks **outstanding work only**. Implemented features are documented
in [README.md](README.md) (usage) and [ARCHITECTURE.md](ARCHITECTURE.md)
(design); shipped changes are recorded in [CHANGELOG.md](CHANGELOG.md).

## Goal

A standalone Rust binary that renders 3D mesh files (STL, OBJ, 3MF) to
deterministic 2D PNG previews or animated GIFs.

```bash
stl-render model.stl -o preview.png --view print --material-color tan
stl-render model.3mf -o preview.gif --animate
```

The rendering pipeline is feature-complete for v1. The only remaining milestone
is publishing the first release.

---

## M16: Release Packaging

**Goal:** Cut the first tagged release with cross-platform binaries and a
crates.io publish.

**Status:** The pipeline is implemented in `.github/workflows/release.yml` — a
tag push builds all six targets below, attaches archives to a GitHub Release,
and runs `cargo publish`. What remains is executing and verifying the first
release.

### Build Targets

| Target | OS | Archive |
|--------|-------|---------|
| `x86_64-unknown-linux-gnu` | ubuntu-latest | .tar.gz |
| `x86_64-unknown-linux-musl` | ubuntu-latest | .tar.gz |
| `aarch64-unknown-linux-gnu` | ubuntu-latest | .tar.gz |
| `x86_64-apple-darwin` | macos-latest | .tar.gz |
| `aarch64-apple-darwin` | macos-latest | .tar.gz |
| `x86_64-pc-windows-msvc` | windows-latest | .zip |

### Release Process

1. Bump `version` in `Cargo.toml` (e.g. `0.2.0`)
2. Move the `CHANGELOG.md` `[Unreleased]` entries under the new version with a date
3. Commit and tag: `git tag v0.2.0`
4. Push: `git push origin main --tags`
5. Workflow builds binaries, creates the release, and publishes to crates.io

### Pre-Release Checklist

- [ ] `cargo publish --dry-run --locked` succeeds (validates packaged crate)
- [ ] All six targets build in CI
- [ ] Downloaded binaries run on each platform

**Acceptance:** Pushing the tag triggers the workflow; all platforms build; the
GitHub Release and crates.io publish both succeed.
