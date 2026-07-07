# First Binary Release (`v0.2.0`) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rework the untracked release workflow into a draft-first, binaries-only pipeline with a dormant crates.io publish job, then cut and publish the first real `v0.2.0` GitHub Release with six platform binaries.

**Architecture:** Tag-triggered GitHub Actions workflow with four jobs — `create-release` (draft + gate), `build` (6-target matrix), `publish-release` (flip draft→public only if all targets pass), `publish-crates` (gated, skipped until a secret exists). Supporting doc/version edits precede the tag push.

**Tech Stack:** GitHub Actions, `softprops/action-gh-release@v2`, `dtolnay/rust-toolchain@stable`, `gh` CLI, Cargo (edition 2024, MSRV 1.88).

**Reference spec:** `docs/superpowers/specs/2026-07-07-first-binary-release-design.md`

---

## File Structure

| File | Change | Responsibility |
|------|--------|----------------|
| `.github/workflows/release.yml` | Rewrite (currently untracked) | The release pipeline: draft-first publish, gated crates.io |
| `Cargo.toml` | Modify line 3 | Version bump `0.1.0` → `0.2.0` |
| `Cargo.lock` | Regenerated | Lockfile package-version entry follows the bump |
| `CHANGELOG.md` | Modify header + footer | Cut `[Unreleased]` → `[0.2.0] - 2026-07-07`, refresh links |
| `PLAN.md` | Rewrite M16 | Milestone = binaries-only release; crates.io deferred |

---

## Task 1: Rework the release workflow

**Files:**
- Modify: `.github/workflows/release.yml` (full rewrite of untracked file)

- [ ] **Step 1: Replace the workflow file with the draft-first, gated design**

Write `.github/workflows/release.yml` with exactly this content:

```yaml
name: Release

on:
  push:
    tags:
      - 'v[0-9]+.*'

permissions:
  contents: write

env:
  CARGO_TERM_COLOR: always

jobs:
  create-release:
    name: Create Release (draft)
    runs-on: ubuntu-latest
    outputs:
      version: ${{ steps.get_version.outputs.version }}
      publish_crates: ${{ steps.crates_gate.outputs.publish }}
    steps:
      - uses: actions/checkout@v4

      - name: Get version from tag
        id: get_version
        run: echo "version=${GITHUB_REF#refs/tags/v}" >> "$GITHUB_OUTPUT"

      - name: Extract changelog for this version
        id: changelog
        run: |
          version="${{ steps.get_version.outputs.version }}"
          sed -n "/^## \[${version}\]/,/^## \[/p" CHANGELOG.md | sed '$d' > release_notes.md
          if [ ! -s release_notes.md ]; then
            echo "Release v${version}" > release_notes.md
          fi

      - name: Determine crates.io publish gate
        id: crates_gate
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
        run: |
          if [ -n "$CARGO_REGISTRY_TOKEN" ]; then
            echo "publish=true" >> "$GITHUB_OUTPUT"
          else
            echo "publish=false" >> "$GITHUB_OUTPUT"
          fi

      - name: Create draft GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          name: v${{ steps.get_version.outputs.version }}
          body_path: release_notes.md
          draft: true
          prerelease: ${{ contains(steps.get_version.outputs.version, '-') }}

  build:
    name: Build ${{ matrix.target }}
    needs: create-release
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            archive: tar.gz
          - target: x86_64-unknown-linux-musl
            os: ubuntu-latest
            archive: tar.gz
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
            archive: tar.gz
          - target: x86_64-apple-darwin
            os: macos-latest
            archive: tar.gz
          - target: aarch64-apple-darwin
            os: macos-latest
            archive: tar.gz
          - target: x86_64-pc-windows-msvc
            os: windows-latest
            archive: zip

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install cross-compilation tools (Linux)
        if: matrix.os == 'ubuntu-latest' && matrix.target != 'x86_64-unknown-linux-gnu'
        run: |
          sudo apt-get update
          case "${{ matrix.target }}" in
            x86_64-unknown-linux-musl)
              sudo apt-get install -y musl-tools
              ;;
            aarch64-unknown-linux-gnu)
              sudo apt-get install -y gcc-aarch64-linux-gnu
              echo "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc" >> "$GITHUB_ENV"
              ;;
          esac

      - name: Build
        run: cargo build --release --locked --target ${{ matrix.target }}

      - name: Package (Unix)
        if: matrix.os != 'windows-latest'
        run: |
          cd target/${{ matrix.target }}/release
          tar czvf ../../../stl-render-${{ needs.create-release.outputs.version }}-${{ matrix.target }}.tar.gz stl-render

      - name: Package (Windows)
        if: matrix.os == 'windows-latest'
        run: |
          cd target/${{ matrix.target }}/release
          7z a ../../../stl-render-${{ needs.create-release.outputs.version }}-${{ matrix.target }}.zip stl-render.exe

      - name: Upload artifact to draft release
        uses: softprops/action-gh-release@v2
        with:
          tag_name: v${{ needs.create-release.outputs.version }}
          draft: true
          files: stl-render-${{ needs.create-release.outputs.version }}-${{ matrix.target }}.${{ matrix.archive }}

  publish-release:
    name: Publish Release
    needs: [create-release, build]
    runs-on: ubuntu-latest
    steps:
      - name: Flip draft to published
        env:
          GH_TOKEN: ${{ github.token }}
        run: gh release edit "v${{ needs.create-release.outputs.version }}" --repo "$GITHUB_REPOSITORY" --draft=false

  publish-crates:
    name: Publish to crates.io
    needs: [create-release, publish-release]
    if: needs.create-release.outputs.publish_crates == 'true'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - name: Publish
        run: cargo publish --locked
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

- [ ] **Step 2: Validate YAML syntax**

Run: `python3 -c 'import yaml; yaml.safe_load(open(".github/workflows/release.yml")); print("yaml ok")'`
Expected: `yaml ok`
(If `ModuleNotFoundError: yaml`, run `python3 -m pip install pyyaml` first, or skip this step — the workflow run will surface syntax errors.)

- [ ] **Step 3: Verify the four expected jobs are present**

Run: `grep -E '^  (create-release|build|publish-release|publish-crates):' .github/workflows/release.yml`
Expected: four lines, one per job name.

- [ ] **Step 4: Commit the workflow**

```bash
git add .github/workflows/release.yml
git commit -m "$(cat <<'EOF'
Rework release workflow: draft-first publish, gated crates.io

Creates the GitHub Release as a draft, attaches all six target binaries,
then flips to published only after every target succeeds. The crates.io
publish job is gated on a CARGO_REGISTRY_TOKEN secret and stays skipped
until that secret is added.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Bump the version and refresh the lockfile

**Files:**
- Modify: `Cargo.toml:3`
- Regenerate: `Cargo.lock`

- [ ] **Step 1: Bump the version in `Cargo.toml`**

Change line 3 from:

```toml
version = "0.1.0"
```

to:

```toml
version = "0.2.0"
```

- [ ] **Step 2: Refresh the lockfile**

Run: `cargo build`
Expected: build succeeds; `Cargo.lock` now records `stl-render` at `0.2.0`.

- [ ] **Step 3: Confirm the lockfile version updated**

Run: `grep -A1 'name = "stl-render"' Cargo.lock`
Expected: the following line reads `version = "0.2.0"`.

(No commit yet — the release-prep edits in Tasks 2–4 are committed together in Task 5 after verification.)

---

## Task 3: Cut the changelog for `0.2.0`

**Files:**
- Modify: `CHANGELOG.md` (header region + link footer)

- [ ] **Step 1: Promote `[Unreleased]` to `[0.2.0]` and add a fresh empty `[Unreleased]`**

Change the header region. Replace:

```markdown
## [Unreleased]

### Added
```

with:

```markdown
## [Unreleased]

## [0.2.0] - 2026-07-07

### Added
```

This keeps all existing entries (they now belong to `0.2.0`) and leaves a new empty `[Unreleased]` section at the top.

- [ ] **Step 2: Update the link footer**

Replace:

```markdown
[Unreleased]: https://github.com/jmkent/stl-render/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/jmkent/stl-render/releases/tag/v0.1.0
```

with:

```markdown
[Unreleased]: https://github.com/jmkent/stl-render/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/jmkent/stl-render/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/jmkent/stl-render/releases/tag/v0.1.0
```

- [ ] **Step 3: Verify the changelog-extraction command produces non-empty notes**

This is the exact logic the workflow runs. Confirm it captures the `0.2.0` body:

Run:
```bash
sed -n "/^## \[0.2.0\]/,/^## \[/p" CHANGELOG.md | sed '$d'
```
Expected: the `## [0.2.0] - 2026-07-07` header followed by the Added/Changed/Fixed entries, and NOT the `## [0.1.0]` header.

---

## Task 4: Update PLAN.md M16

**Files:**
- Modify: `PLAN.md` (M16 section)

- [ ] **Step 1: Replace the M16 section**

Replace the entire `## M16: Release Packaging` section (from that heading through the end of the file) with:

```markdown
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
```

---

## Task 5: Pre-flight verification and commit the release prep

**Files:** none modified (verification + commit of Tasks 2–4)

- [ ] **Step 1: Format check**

Run: `cargo fmt --check`
Expected: no output, exit 0.

- [ ] **Step 2: Lint**

Run: `cargo clippy -- -D warnings`
Expected: `Finished` with no warnings, exit 0.

- [ ] **Step 3: Tests**

Run: `cargo test`
Expected: all tests pass.

- [ ] **Step 4: Release build**

Run: `cargo build --release`
Expected: `Finished \`release\` profile` build, exit 0.

- [ ] **Step 5: Packaged-crate dry run**

Run: `cargo publish --dry-run --locked`
Expected: packaging succeeds (`Packaged N files`), exit 0. This validates the crate would publish cleanly even though we are not publishing.

- [ ] **Step 6: Commit the release prep**

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md PLAN.md
git commit -m "$(cat <<'EOF'
Prepare v0.2.0 release

Bump version to 0.2.0, cut the changelog for 0.2.0 (2026-07-07), and
update PLAN.md M16 to reflect the binaries-only release with crates.io
deferred.

EOF
)"
```

---

## Task 6: Tag, push, and verify the release

**Files:** none (release execution)

- [ ] **Step 1: Confirm a clean tree on `main` at the prep commit**

Run: `git status --short && git log --oneline -3`
Expected: clean tree; top two commits are the release-prep and workflow-rework commits.

- [ ] **Step 2: Create and push the tag**

```bash
git tag v0.2.0
git push origin main --tags
```
Expected: `main` and `v0.2.0` push to `origin` (`jmkent/stl-render`).

- [ ] **Step 3: Monitor the workflow run**

Open `https://github.com/jmkent/stl-render/actions` (or `gh run watch` if `gh` is installed locally).
Expected job sequence:
1. `create-release` succeeds → a **draft** Release `v0.2.0` exists.
2. All six `build` legs succeed and attach one archive each.
3. `publish-release` succeeds → the Release is no longer a draft.
4. `publish-crates` shows **Skipped** (no `CARGO_REGISTRY_TOKEN` secret).

- [ ] **Step 4: Verify the published release**

Open `https://github.com/jmkent/stl-render/releases/tag/v0.2.0`.
Expected: published (not draft), release notes from the `0.2.0` changelog section, and six attached archives (five `.tar.gz`, one `.zip`).

- [ ] **Step 5: Smoke-test the shipped macOS binary**

```bash
cd $(mktemp -d)
curl -sSL -o stl-render.tar.gz \
  https://github.com/jmkent/stl-render/releases/download/v0.2.0/stl-render-0.2.0-aarch64-apple-darwin.tar.gz
tar xzf stl-render.tar.gz
./stl-render --version
./stl-render ~/3dprinting/3dbenchy/files/3DBenchy.stl -o /tmp/benchy-release-check.png --view print
```
Expected: `--version` prints `stl-render 0.2.0`; the render writes `/tmp/benchy-release-check.png` with no error. (The 3DBenchy STL is local-only at that path.)

---

## Rollback / Failure Handling

- **A build target fails:** `publish-release` never runs, so the Release stays a private draft — nothing public is partial. Delete the draft in the GitHub UI, delete the tag (`git push origin :refs/tags/v0.2.0` and `git tag -d v0.2.0`), fix the cause, and re-tag.
- **`cargo publish --dry-run` fails in Task 5:** do not tag. Fix packaging (usually a missing/excluded file or metadata) and re-run Task 5.
- **Wrong release notes:** the notes come from the `0.2.0` changelog section; fix `CHANGELOG.md`, and edit the GitHub Release body directly (already published) or delete-draft-and-retag if caught before publish.
