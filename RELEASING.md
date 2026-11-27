# Releasing

This repository publishes one crate to crates.io: `wesley-rust-syntax`. `wesley-emit-rust` is `publish = false`; its next release belongs to Wesley's major-version change (flyingrobots/wesley#882), and the release workflow names `-p wesley-rust-syntax` on every `cargo publish`, so it cannot publish anything else.

A release is an annotated tag `v<version>` on `main`, where `<version>` is the `version` in `crates/wesley-rust-syntax/Cargo.toml`. Pushing the tag runs [`.github/workflows/release.yml`](.github/workflows/release.yml); nothing reaches crates.io until the owner approves the `crates-io` deployment.

## 1. Merge the release-prep pull request

The release-prep pull request:

- sets `version` in `crates/wesley-rust-syntax/Cargo.toml`;
- sets `wesley-emit-rust`'s path dependency on it to the same exact version (`wesley-rust-syntax = { path = "../wesley-rust-syntax", version = "=<version>" }` in `crates/wesley-emit-rust/Cargo.toml`), because Cargo checks that requirement against the local crate and the workspace does not build while it names the old version;
- refreshes `Cargo.lock` (`cargo update -p wesley-rust-syntax`) so that `--locked` passes, and commits it;
- turns `## Unreleased` in [CHANGELOG.md](CHANGELOG.md) into `## [<version>] - <date>` with an empty `## Unreleased` above it.

It must pass `checks`, which runs `cargo publish --dry-run --locked -p wesley-rust-syntax` and `scripts/check-package.sh`; the script fails when a workspace crate's path dependency on `wesley-rust-syntax` does not pin exactly its version. Merge it with a merge commit.

## 2. Tag `main` and push the tag

Tag the merge commit on an up-to-date `main` with an annotated tag, then push only that tag:

```sh
git switch main
git pull --ff-only
git tag -a v0.1.0-alpha.1 -m "wesley-rust-syntax 0.1.0-alpha.1"
git push origin v0.1.0-alpha.1
```

The `gate` job refuses a lightweight tag, a tag whose commit is not on `origin/main`, a tag that differs from the crate's `version` (read with `cargo metadata`), and a version with no `## [<version>]` heading in CHANGELOG.md. It then runs `cargo fmt --check`, `cargo clippy -D warnings`, the full test suite with `--locked`, `scripts/check-package.sh` (the `.crate` holds the README, `LICENSE` and `NOTICE`) and `cargo publish --dry-run --locked -p wesley-rust-syntax`. A run started by hand with `workflow_dispatch` is refused unless its ref is a tag.

## 3. Approve the `crates-io` deployment

The `publish` job waits in the `crates-io` environment, which only `v*` tags may deploy to and which needs the owner's approval. Approve it from the workflow run's page. The job checks that the tag still points at the commit the gate checked, then runs `cargo publish --locked -p wesley-rust-syntax` with the `CARGO_REGISTRY_TOKEN` secret. If crates.io already has the version (a re-run after a later job failed), it publishes nothing and succeeds.

## 4. What `verify` checks

The `verify` job polls `cargo info wesley-rust-syntax@<version> --registry crates-io` up to 20 times, 15 seconds apart, until crates.io serves the version. The crate has no binary, so nothing is installed; instead the job creates a fresh library crate outside the repository, depends on `wesley-rust-syntax = "=<version>"` from crates.io, and runs a test that calls `naming::type_name`. This proves the published crate resolves and compiles on its own.

The `github-release` job then creates the GitHub release for the tag with `gh release create --verify-tag`, using that version's CHANGELOG section as its notes, marked as a pre-release when the version has a `-` suffix. It is the only job with `contents: write`.

## If a step fails

Every job is safe to re-run. Fix what the gate refused on `main`, then release under a new version: never move or re-push a tag that has been pushed. If `verify` or `github-release` failed after a successful publish, re-run the workflow; `publish` sees the version on crates.io and skips.

## Yanking

crates.io never deletes or replaces a published version. If a release is broken, yank it so that new lockfiles stop selecting it, while builds that already lock it keep working:

```sh
cargo yank --version 0.1.0-alpha.1 wesley-rust-syntax
```

Undo a yank with `cargo yank --undo --version 0.1.0-alpha.1 wesley-rust-syntax`. Publish the fix as a new version through this procedure, record the yank in CHANGELOG.md, and edit the GitHub release to say it was yanked.
