AGENTS.md for agentic contributors

Purpose
- This document tells automated agents (and humans) how to build, lint, test, and follow code style in this repository.
- Scope: repository root and sub-crates `kernel/` and `user/`.

Quick links
- Plan and TODO manifest: `PLAN.org`
- Workspace Cargo manifest: `Cargo.toml`
- Kernel crate: `kernel/Cargo.toml`
- User crate: `user/Cargo.toml`
- Kernel Makefile: `kernel/Makefile`
- Root Makefile: `Makefile`
- VSCode settings: `.vscode/settings.json`

Environment
- This repository targets `riscv64gc-unknown-none-elf` for the build. See `.cargo/config` for the default `target`.
- The project uses edition 2021 for Rust and includes `rust-toolchain.toml` pinned toolchain.
- Many development commands are wrapped in `Makefile` targets so prefer `make ...` for common flows when available.

Build, Lint, Test commands
- Build kernel (release, cross-compiled):
  - `make kernel`
  - Under the hood this uses `cargo rustc --target=riscv64gc-unknown-none-elf --release -p ruscv_kernel` with link flags from `kernel/src/kernel.ld`.
- Build user apps (all user bins):
  - `make user`
  - Invokes `cargo rustc -p user_lib` with user linker script `user/src/linker.ld`.
- Run QEMU (boot the kernel):
  - `make qemu` (launches `qemu-system-riscv64` with the built kernel and rustsbi)
- Launch LLDB attached to QEMU (for debugging):
  - `make lldb`
- Run clippy (lint):
  - `make clippy`
  - Equivalent low-level command: `RUSTFLAGS="$(RUSTFLAGS_PEDANTIC)" cargo clippy --target=riscv64gc-unknown-none-elf --release`
- Format code:
  - `cargo fmt --all` — recommended to run before commits.
- Run docs:
  - `cargo doc --all --no-deps` (cross-target docs may not be useful for no_std crates).

Notes about tests in this workspace
- This is a no_std, cross-compiled kernel workspace. Unit/integration tests that rely on std or host platform are generally not supported for `kernel` and may not compile or run for the RISC-V target.
- If a crate contains host-runnable tests (for example helper crates built for the host), run tests with `cargo test` targeting the host platform.
- Common test commands (host-local crates):
  - Run all tests in a package: `cargo test -p <package>` (example: `cargo test -p user_lib`)
  - Run a single test by name (filter): `cargo test -p <package> <test_name_filter> -- --nocapture --test-threads=1`
    - Example: `cargo test -p user_lib my_test_function -- --nocapture --test-threads=1`
  - Run a single test file/integration test: `cargo test --test <test_filename_without_ext> -p <package>`
- Single-test tips for cross-target crates:
  - If a crate is cross-compiled (RISC-V) but you want to run a unit test on the host, you may need to make the crate testable on the host by adding `cfg(test)`-only `std` features or by creating a small host-side test harness crate.
  - Agents must not assume `cargo test` will work for `kernel/` without adapting the crate to the host environment.

Common helper commands
- Show build artifacts: `ls target/riscv64gc-unknown-none-elf/release`
- Generate stripped binary: `rust-objcopy --strip-all -O binary target/.../kernel kernel.bin` (already handled by Makefile targets)
- Inspect binary: `llvm-readelf --headers <path-to-elf>`, `llvm-objdump --disassemble --demangle --section .text <elf>`
- Run clippy with pedantic flags (recommended for CI): `RUSTFLAGS="$(RUSTFLAGS_PEDANTIC)" cargo clippy --all-targets --all-features -- -D warnings`

Repository conventions and style
- General
  - Keep changes minimal and targeted. When modifying existing files, prefer small, well-scoped edits.
  - Use the Rust 2021 edition idioms and prefer explicitness.
  - Follow the principle: fix root cause, not symptoms.

- Formatting
  - Run `cargo fmt --all` before committing. Configure editor to auto-format on save where possible (this repo's `.vscode/settings.json` enables `editor.formatOnSave`).
  - Line width: follow the project's VSCode ruler at 100 columns.

- Imports
  - Do not use glob imports (`use foo::*`) except in short test modules where convenience outweighs clarity.
  - Order imports in this sequence: external crates (sorted), `core`/`alloc`/`std` (if used), then workspace crates (absolute paths starting with crate or crate name), then `super`/`self`/local module imports. Keep groups separated by a blank line.
  - Prefer absolute paths (e.g., `crate::memory::PageTable`) for public API items.

- Types and naming
  - Functions & variables: `snake_case`
  - Types (struct/enum/trait): `CamelCase`
  - Constants and statics: `SCREAMING_SNAKE_CASE` (for compile-time constants) or `PascalCase` for public type names
  - Module names: `snake_case`
  - Use descriptive names; avoid one-letter names except for short loop indices.

- Error handling
  - Prefer `Result<T, E>` and the `?` operator for fallible functions. Return early when appropriate.
  - Avoid `.unwrap()` and `.expect()` in library/kernel code. If `unwrap` is used, it must include a clear message and be justified in a comment (only acceptable in prototypes or tests).
  - In low-level kernel code where errors cannot be propagated, handle faults explicitly and document the reasoning in a safety comment.
  - Define small, focused error types rather than propagating broad error enums across modules.

- Unsafe and no_std rules
  - No_std policy: `kernel/` is designed for no_std. Avoid using std or dependencies that require std.
  - Mark `unsafe` blocks with a short, clear safety comment explaining invariants that must hold (what is being assumed and why it is safe).
  - Keep unsafe blocks as small as possible and prefer safe abstractions outside the block.
  - For FFI and assembly (`.s` files), ensure `#[repr(C)]` on structs shared with assembly and document the expected layout and alignment. Reference: `kernel/src/entry.s`, `kernel/src/process/switch.s`.

- Concurrency and synchronization
  - Prefer explicit synchronization primitives provided in `kernel/src/sync/`.
  - Avoid data races and leaking interior mutability without synchronization. Use `spin`locks or atomic types as appropriate for the environment.

- Memory and allocation
  - Kernel heap code is centralized under `kernel/src/kernel_heap`. Prefer those abstractions rather than ad-hoc allocators.
  - When working with physical addresses, frames, or page tables (`kernel/src/memory/`), document address-space invariants and alignment requirements clearly.

- Assembly and linker scripts
  - Keep linker scripts (`kernel/src/kernel.ld`, `user/src/linker.ld`) authoritative for symbol and memory layout. Do not duplicate layout rules in code without cross-reference comments.
  - Assembly files (ending in `.s`) must be kept minimal; all complex logic should be in Rust. Document the calling convention and register usage.

- Logging and feature flags
  - Logging features are gated by features in `kernel/Cargo.toml` (e.g., `log-error`, `log-warn`, `log-info`, ...). Prefer using these features for dev vs release verbosity.
  - Use `KERNEL_LOG` env var for `Makefile` builds to set default log level (e.g., `make kernel KERNEL_LOG=TRACE`).

- Tests and CI
  - There are no repository-level CI files here; when adding CI ensure cross-compilation/host-test distinctions are explicit.
  - If you add tests to `kernel/`, document how to run them locally (host harness or emulator) since `cargo test` will not run on a `none-elf` target by default.

Editor/IDE guidance
- The workspace suggests using `rust-analyzer` with `riscv64gc-unknown-none-elf` configured in `.vscode/settings.json` for target-aware analysis of the kernel crate.
- Use the pinned toolchain (`rust-toolchain.toml`) to ensure reproducible builds.

Cursor / Copilot rules
- Cursor rules: none found in `.cursor/rules/` or `.cursorrules`.
- GitHub Copilot instructions: none found at `.github/copilot-instructions.md`.
- If you want agents to follow specific Cursor or Copilot rules, add them to one of the above locations and mention them in this file.

Guidance for automated agents (this is important)
- Read before editing: Always `git status` and `git diff` to see local state before changing files.
- Safety first: For any change touching `unsafe`, memory layout, or linker scripts, require an explicit human review step and add detailed safety comments.
- Tests: Do not assume `cargo test` succeeds for `kernel/`. If you add tests, include a host-run harness or a QEMU-based integration test and document the exact commands.
- Commits and pushes: Agents must not create commits or push to remotes unless explicitly requested by a human operator.
- When editing files, obey repository style in this document: small, focused edits; run `cargo fmt`; run `make clippy` if you changed logic.

If anything here is unclear
- Ask for clarification before making changes that affect low-level invariants (memory layout, unsafe code, ABI, linker scripts, or scheduler/synchronization primitives).

Changelog
- 2026-03-30: Initial AGENTS.md created by assistant. Includes build/lint/test commands, style guide tuned for no_std RISC-V kernel workspace, and guidance for automated agents.

References
- `.cargo/config` (target) — see `/.cargo/config:1`
- `kernel/Cargo.toml` (features, crate name) — see `kernel/Cargo.toml`
- `Makefile` and `kernel/Makefile` for common targets and usage.

**Run QEMU**
- Default `make qemu` behavior: the top-level `Makefile` launches QEMU with the `rustsbi` BIOS and the kernel binary. By default it configures serial to a telnet server on port `1235`.

- Quick ways to view serial output while `make qemu` runs:
  - Connect via `nc` (telnet) to the serial port (recommended):
    - Start QEMU: `make qemu`
    - In another terminal run: `nc localhost 1235`
  - Log serial to a file (non-interactive):
    - Run QEMU with serial redirected to a file and tail it:
      - `qemu-system-riscv64 -M virt -s -S --nographic -cpu rv64 -smp 1 -net none -bios rustsbi/rustsbi-qemu -serial file:serial.log -device loader,file=kernel.bin,addr=0x80200000`
      - Then in another terminal: `tail -f serial.log`
  - Makefile change for serial on stdio (single terminal):
    - Edit `Makefile` and replace `-serial telnet::1235,server` with `-serial stdio` in `QEMU_CMD`.
    - Then `make qemu` will print serial output to the terminal. Note: `-serial stdio` can fail if multiple devices attempt to use stdio (the current Makefile may start other character devices), so use with caution.

- Troubleshooting
  - If `nc` cannot connect, ensure QEMU is still running and listening on port `1235` (QEMU prints the listening line on startup). If QEMU exits immediately, check `make qemu` logs for errors or verify the kernel binary (`kernel.bin`) exists and is valid.
  - If the serial log file is empty, QEMU may not have produced output yet or may have exited; run `tail -n +1 serial.log` after QEMU runs to see any early output.

**Adding rustsbi prototyper**
- The project can use the RustSBI Prototyper as the firmware (BIOS) for QEMU (see https://github.com/rustsbi/rustsbi/prototyper).
- Required packages (ensure they are present in the dev shell):
  - `cargo-binutils` (already included in the flake)
  - `uboot-tools` (added as `ubootTools` in `flake.nix` buildInputs)
- Makefile helper target: `make rustsbi-prototyper`
  - This target will `git clone` the `rustsbi` repo into `./rustsbi`, build the prototyper in `rustsbi/prototyper/prototyper`, and copy the built binary to `target/rustsbi/`.
- How to run QEMU with the built prototyper as BIOS:
  - After building, run QEMU using the prototyper ELF as BIOS:
    - `qemu-system-riscv64 -machine virt -bios target/rustsbi/rustsbi-prototyper-dynamic.elf -display none -serial stdio`
  - Or update `Makefile`'s `RUSTSBI_BIN` to point to `target/rustsbi/rustsbi-prototyper-dynamic.elf` and then `make qemu`.

Notes
- Building rustsbi may download many git dependencies; ensure you have working network access and allow `cargo` to fetch crates and git repositories.
- In this environment the build failed due to network timeouts while fetching git dependencies. Locally with network access it should succeed.

End of AGENTS.md
