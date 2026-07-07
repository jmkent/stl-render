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

**Goal:** Cut the first tagged release with cross-platform binaries. crates.io
publishing is deferred.

**Status:** The pipeline in `.github/workflows/release.yml` builds all six targets
below, creates a **draft** GitHub Release, attaches the archives, and flips the
release to published only after every target succeeds. A `publish-crates` job is
wired but gated on a `CARGO_REGISTRY_TOKEN` secret, so it stays skipped until that
secret is added. What remains is executing and verifying the first release.

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

1. Bump `version` in `Cargo.toml` (e.g. `0.2.0`) and refresh `Cargo.lock`
2. Move the `CHANGELOG.md` `[Unreleased]` entries under the new version with a date
3. Commit, then tag: `git tag v0.2.0`
4. Push: `git push origin main --tags`
5. Workflow builds binaries, creates the draft, attaches archives, and publishes

### Enabling crates.io later

Add a `CARGO_REGISTRY_TOKEN` repository secret. The `publish-crates` job's gate then
evaluates true and it runs on the next tagged release — no workflow edit required.

### Pre-Release Checklist

- [ ] `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` pass
- [ ] `cargo publish --dry-run --locked` succeeds (validates the packaged crate)
- [ ] All six targets build in CI
- [ ] Downloaded binary runs on this machine

**Acceptance:** Pushing the tag triggers the workflow; all six platforms build; the
draft flips to a published GitHub Release with all archives attached; `publish-crates`
is skipped.
