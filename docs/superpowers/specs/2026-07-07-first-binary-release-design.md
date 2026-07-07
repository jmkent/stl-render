# First Binary Release (`v0.2.0`) — Design

**Date:** 2026-07-07
**Status:** Approved for planning

## Summary

`stl-render` is feature-complete for v1 but has never published built binaries. An
untracked `.github/workflows/release.yml` already builds six cross-platform targets,
creates a GitHub Release, and runs `cargo publish`. This work reworks that workflow
into a robust, **binaries-only** release pipeline, aligns the docs, and drives the
first real `v0.2.0` release through to a published GitHub Release with binaries
attached.

crates.io publishing is **deferred, not removed**: the job stays wired but is gated
so it is skipped until a `CARGO_REGISTRY_TOKEN` secret exists.

## Goals

- A tag push (`v[0-9]+.*`) produces a published GitHub Release with all six
  platform archives attached.
- No public release is ever left in a partial state (missing/incomplete binaries).
- crates.io publish is dormant now and flippable later without editing the workflow.
- Cut and publish the actual `v0.2.0` release.

## Non-Goals

- Publishing to crates.io in this release.
- Adding new rendering features or changing the CLI.
- Homebrew/apt/other package-manager distribution.

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Session outcome | Design the release path **and** cut `v0.2.0` | User wants the first release actually shipped. |
| crates.io job | Gate it, keep dormant | "Not initially" implies later; avoid re-adding YAML from history. |
| Publish flow | Draft first, publish last | First-release robustness; no public partial artifacts. |
| Version | `0.2.0` | Still `0.x`; the `[Unreleased]` block is additive. |
| Release source | `main`, tag on the prep commit | Standard flow. |

## Workflow Design (`.github/workflows/release.yml`)

Tag-triggered on `v[0-9]+.*`. Four jobs:

### 1. `create-release`
- Checks out, derives `version` from the tag.
- Extracts this version's section from `CHANGELOG.md` into release notes
  (existing `sed` logic, unchanged).
- Creates the GitHub Release as a **draft** (`draft: true`) so nothing is public yet.
- **New:** a step reads `CARGO_REGISTRY_TOKEN` into an env var and emits a
  `publish` output (`true`/`false`). This indirection is required because the
  `secrets` context cannot be referenced directly in a job-level `if:`.
- Outputs: `version`, `publish`.

### 2. `build` (matrix ×6)
- `needs: create-release`, `fail-fast: false`.
- Targets unchanged:
  `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`,
  `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`,
  `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`.
- Builds `--release --locked --target <t>`, packages `.tar.gz` (Unix) / `.zip`
  (Windows), uploads the archive to the draft release via `tag_name`.

### 3. `publish-release` (new)
- `needs: [create-release, build]` — because `needs` requires *all* matrix legs
  to succeed, this runs only when every target built and attached.
- Flips the release from draft to public (`draft: false`) via the same
  `softprops/action-gh-release@v2` action against the existing tag.

### 4. `publish-crates`
- `needs: [create-release, publish-release]`.
- Gated: `if: needs.create-release.outputs.publish == 'true'`.
- With no secret set, the gate is `false` and the job is **skipped**.
- Adding `CARGO_REGISTRY_TOKEN` later makes it run — no workflow edit needed.

### Failure semantics
Any target failing → `publish-release` does not run → the release stays a private
draft with no partial public artifacts. The draft can be inspected and deleted
manually, then the tag re-pushed after a fix.

## Release-Prep Changes (prerequisites to tagging)

1. `Cargo.toml`: `version = "0.1.0"` → `"0.2.0"`; refresh `Cargo.lock` by running
   `cargo build` (or `cargo check`), which updates the lockfile's own package
   version entry to `0.2.0`.
2. `CHANGELOG.md`: move the `[Unreleased]` block under `## [0.2.0] - 2026-07-07`,
   add a fresh empty `[Unreleased]` section, and update the compare/link footer
   (`[Unreleased]` → compare `v0.2.0...HEAD`; add `[0.2.0]` tag link).
3. `PLAN.md`: rewrite M16 — the milestone is the binaries-only release; crates.io
   publish is documented as a deferred, dormant step enabled by adding the secret.

## Pre-Flight Verification (local, before tagging)

All must pass green:

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`
- `cargo build --release`
- `cargo publish --dry-run --locked` (validates the packaged crate; does not upload)

## Execution & Post-Release Verification

1. Commit the prep changes and the reworked workflow to `main`.
2. `git tag v0.2.0 && git push origin main --tags`.
3. Monitor the Actions run and confirm the sequence:
   draft created → six binaries built → release flipped public → `publish-crates`
   **skipped**.
4. Download the `aarch64-apple-darwin` archive, extract, and run it against a real
   model (e.g. `stl-render --version` and a render) to confirm the shipped binary
   works on this machine.

## Testing

The workflow is only exercisable by tagging, so verification is the monitored first
run plus the downloaded-binary smoke check. No unit tests apply to this change.
