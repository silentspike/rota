# ROTA — Hinweise für Agenten

Die Projektregeln stehen in [CLAUDE.md](CLAUDE.md). Diese Datei trägt nur den Abschnitt zu
cargo remote v2, und den verwaltet dessen Client: Er ersetzt alles zwischen den beiden Marken
durch seine Vorlage. Deshalb steht die Datei in `.prettierignore` — eine Umformatierung hielte
nur bis zum nächsten Abgleich.

<!-- cargo-remote-v2:begin -->
## Cargo Remote

Run Rust builds, tests, Clippy and formatting on the remote build service.
Start commands from the Cargo project or workspace root.

The project opts into v2 through `[cargo_remote]` with `schema = 1` in
`.cargo-remote.toml`. Existing unmigrated projects and legacy `[[remote]]`
configurations keep the old service until explicitly migrated. **The old service
still requires serialized heavy builds.** V2 permits parallel builds in separate
checkouts/worktrees; its server reserves capacity before uploading any source.
Each workspace permits one active job. There is no build queue.

- Build: `cargo remote -c -- build`, or `cargo remote -c -- build --release`.
- Test: `cargo remote -- test --workspace --no-fail-fast`.
- Clippy: `cargo remote -- clippy --workspace --all-targets -- -D warnings`.
- Format check: `cargo-remote-fmt --check`; apply: `cargo-remote-fmt`.
- Large build: `cargo remote --resources=large -c -- build --release`.
- Status: `cargo remote --status`; logs: `cargo remote --logs JOB_ID`.
- Recover artifacts after a disconnection: `cargo remote --resume JOB_ID`.
- Cancel your own retained job: `cargo remote --cancel JOB_ID`.
- Show the installed contract: `cargo remote --help`.

Normal jobs reserve 4 GiB and use two Cargo compiler jobs; large jobs reserve
12 GiB and use four. The aggregate reservation budget is 20 GiB. Exit 75 means
capacity or a workspace is unavailable: no job was queued. Inspect status before
retrying; do not duplicate a disconnected job or bypass resource limits.

New projects initialize automatically on first use, with a pinned toolchain,
small versioned project configuration and a managed instruction section.
For an existing project, use `cargo remote --init --profile PROFILE` only after
its deployment target has been checked. Profiles include `linux-gnu-bookworm`,
`desktop-linux`, `desktop-suse-ffmpeg7`, `android-r27d` and `wasm`. The SUSE FFmpeg
profile uses an explicitly enrolled target SDK. Desktop native libraries must match the
deployment target. Android requires its Android target. Specialized targets need
their own audited profile. Existing exact Rust pins are preserved; the audited
old stable is 1.98.1, and genuinely new projects use 1.99.0. `-d VERSION` changes
only that invocation and does not change the global rustup default.

`-c` copies emitted project executables and deployable libraries, not every
dependency or the whole target tree. Use `--copy-target` only when the full tree
is required. Checks and remote functional tests generally need no `-c`.
`--no-copy-lock` prevents Cargo.lock copy-back. Rustfmt apply copies changed Rust
source back only if the local snapshot is still unchanged; conflicts fail closed.
Check Cargo.lock and generated-file diffs after every run.

Verify exit codes and test counts: a filter matching zero tests is not acceptance.
Functional tests can run remotely; hardware/performance claims and deployment
acceptance require the actual target. Source credentials, SSH agents, production
databases and secret environment variables must not be sent to build jobs.
Compiler cache, job limits and source hash caching are automatic.

These instructions are maintained from one versioned template. Existing running
agents must explicitly reread updated instructions; new sessions load their
applicable global and project instructions normally.
<!-- cargo-remote-v2:end -->
