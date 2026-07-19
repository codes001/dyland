# Story 1.1: Project Scaffolding & Tooling

Status: review

## Story

As a developer,
I want the project initialized as a monorepo with Cargo and npm workspaces, build tooling, linting, formatting, and single-command dev workflow
so that the codebase structure is established from day one with efficient development experience.

## Acceptance Criteria

1. `cargo build` compiles `dyd-daemon` binary
2. `npm install` resolves all frontend dependencies
3. Workspace members defined in both `Cargo.toml` (virtual manifest) and `package.json` (npm workspaces)
4. Shared types directory exists for Rust/TypeScript type definitions
5. Single command (`make dev` or `npm run dev`) starts both daemon and UI with hot-reload
6. `cargo clippy` passes with no warnings
7. `cargo fmt` passes
8. ESLint passes with project-specific ruleset
9. Prettier passes for TypeScript/Tailwind/CSS/JSON
10. GitHub Actions workflow triggers on PR and push to main, completes in under 5 minutes

## Tasks / Subtasks

- [x] Task 1: Initialize monorepo structure (AC: 1, 2, 3, 4)
  - [x] Create root `Cargo.toml` virtual workspace with members `["dyd-daemon", "types"]`
  - [x] Create root `package.json` with workspaces `["dyd-ui"]`
  - [x] Create `dyd-daemon/Cargo.toml` with dependencies (tokio, zbus, serde, tracing)
  - [x] Create `dyd-ui/` directory with Vite + React + TypeScript + Tailwind setup
  - [x] Create `types/` directory with Rust (`types/src/lib.rs`) and TypeScript (`types/src/index.ts`) definitions
  - [x] Create `.gitignore` for Rust and Node artifacts
- [x] Task 2: Set up shared type system (AC: 4)
  - [x] Define basic IPC envelope types in Rust (`types/src/lib.rs`)
  - [x] Define matching TypeScript types in `types/src/index.ts`
  - [x] Add `types` crate as dependency to `dyd-daemon`
  - [x] Create build script to sync types if needed (manual sync for now)
- [x] Task 3: Implement dev workflow (AC: 5)
  - [x] Create `Makefile` with targets: `dev`, `build`, `lint`, `test`, `clean`
  - [x] Install and configure `concurrently` for parallel process management
  - [x] Wire `make dev` to start both `dyd-daemon` (cargo run) and `dyd-ui` (vite dev)
  - [x] Add daemon stdout prefix `[daemon]` and UI prefix `[ui]`
  - [x] Daemon hot-reload via `cargo watch` or equivalent (documented, optional install)
  - [x] Vite HMR for frontend changes
- [x] Task 4: Configure linting & formatting (AC: 6, 7, 8, 9)
  - [x] Create `clippy.toml` with project-specific lint configuration
  - [x] Add Rust lint scripts to `Makefile`
  - [x] Install ESLint with traditional config for TypeScript + React
  - [x] Create `.prettierrc` with Tailwind plugin for class sorting
  - [x] Add pre-commit hook configuration (optional, documented)
- [x] Task 5: Configure CI (AC: 10)
  - [x] Create `.github/workflows/ci.yml`
  - [x] Add Rust build, clippy, fmt --check, test steps
  - [x] Add npm ci, lint, typecheck, build steps
  - [x] Configure `Swatinem/rust-cache` and `actions/cache` for npm
  - [x] Add status badge in README

## Dev Notes

- Root `Cargo.toml` virtual workspace — no `[package]`, only `[workspace]`
- `dyd-daemon` uses tokio async runtime, zbus for D-Bus, serde for serialization, tracing for logging
- `dyd-ui` uses Vite + React 18+ + TypeScript + Tailwind CSS v3+ + Motion (framer-motion)
- Types directory: Rust lib crate + TypeScript module, both defining the IPC protocol envelope
- Dev workflow: `concurrently` npm package spawns both processes with labeled output
- CI: GitHub Actions with proper caching for fast runs
- Use `cargo-watch` for Rust auto-recompilation on file changes (optional install)
- ESLint with traditional `.eslintrc.cjs` config for TypeScript + React (eslint v8 compatibility)
- Prettier with Tailwind plugin for consistent class ordering

### Project Structure

```
dyland/
├── Cargo.toml              # Virtual workspace
├── package.json            # npm workspaces
├── Makefile                # Dev workflow
├── types/
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs          # Rust type definitions
│   └── src/
│       └── index.ts        # TypeScript type definitions
├── dyd-daemon/
│   ├── Cargo.toml
│   └── src/
│       └── main.rs         # Entry point
├── dyd-ui/
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── tailwind.config.js
│   ├── index.html
│   └── src/
│       ├── main.tsx
│       ├── App.tsx
│       └── index.css
├── .github/
│   └── workflows/
│       └── ci.yml
├── .gitignore
├── clippy.toml
├── .eslintrc.cjs
└── .prettierrc
```

### References

- [Source: bmad-output/EPIC-01-FOUNDATION.md#Feature-1.1]
- [Source: bmad-output/SYSTEM-DESIGN.md#Architecture-Overview]
- [Source: bmad-output/IMPLEMENTATION-STRATEGY.md#Phase-1-Skeleton]

## Dev Agent Record

### Agent Model Used

deepseek-v4-flash-free

### Debug Log References

- Rust toolchain: rustc 1.97.1, installed via rustup
- zbus v4.4.0 used (latest v5.x had breaking API changes)
- npm dependencies resolved for eslint v8 compatibility (flat config not used)

### Completion Notes List

- ✅ Monorepo structure created with Cargo virtual workspace + npm workspaces
- ✅ dyd-daemon binary compiles with tokio, zbus, serde, tracing dependencies
- ✅ dyd-ui frontend builds with Vite + React 18 + TypeScript + Tailwind
- ✅ Shared types crate (Rust) and module (TypeScript) with IPC envelope definitions
- ✅ Makefile with dev/build/lint/test/clean targets
- ✅ concurrently configured for parallel daemon+UI startup with labeled output
- ✅ clippy.toml with project-specific lint configuration
- ✅ ESLint (v8) with TypeScript + React plugins, passes on current code
- ✅ Prettier with Tailwind plugin, passes on current code
- ✅ GitHub Actions CI workflow with Rust and frontend jobs
- ✅ .gitignore for Rust and Node artifacts

### File List

New files (relative to project root):
- `Cargo.toml` — Virtual workspace manifest
- `package.json` — npm workspaces root
- `Makefile` — Dev workflow targets
- `.gitignore` — Rust + Node ignores
- `clippy.toml` — Clippy configuration
- `.eslintrc.cjs` — ESLint config
- `.prettierrc` — Prettier config
- `types/Cargo.toml` — Types crate manifest
- `types/src/lib.rs` — Rust IPC type definitions
- `types/src/index.ts` — TypeScript IPC type definitions
- `dyd-daemon/Cargo.toml` — Daemon crate manifest
- `dyd-daemon/src/main.rs` — Daemon entry point
- `dyd-ui/package.json` — UI package manifest
- `dyd-ui/tsconfig.json` — TypeScript config
- `dyd-ui/vite.config.ts` — Vite config
- `dyd-ui/tailwind.config.js` — Tailwind config
- `dyd-ui/postcss.config.js` — PostCSS config
- `dyd-ui/index.html` — HTML entry point
- `dyd-ui/src/main.tsx` — React entry point
- `dyd-ui/src/App.tsx` — Root React component
- `dyd-ui/src/index.css` — Global styles with Tailwind
- `dyd-ui/src/vite-env.d.ts` — Vite type declarations
- `.github/workflows/ci.yml` — CI workflow

New directories:
- `types/src/`
- `dyd-daemon/src/`
- `dyd-ui/src/`
- `.github/workflows/`

## Change Log

- 2026-07-19: Initial story creation from Epic 1 Feature 1.1
- 2026-07-19: Full implementation completed — monorepo structure, shared types, dev workflow, linting/formatting, CI

## Status

Review — Story implementation complete, awaiting code review.
