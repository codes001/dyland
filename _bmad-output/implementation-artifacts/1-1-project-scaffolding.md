# Story 1.1: Project Scaffolding & Tooling

Status: ready-for-dev

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

- [ ] Task 1: Initialize monorepo structure (AC: 1, 2, 3, 4)
  - [ ] Create root `Cargo.toml` virtual workspace with members `["dyd-daemon", "types"]`
  - [ ] Create root `package.json` with workspaces `["dyd-ui"]`
  - [ ] Create `dyd-daemon/Cargo.toml` with dependencies (tokio, zbus, serde, tracing)
  - [ ] Create `dyd-ui/` directory with Vite + React + TypeScript + Tailwind setup
  - [ ] Create `types/` directory with Rust (`types/src/lib.rs`) and TypeScript (`types/src/index.ts`) definitions
  - [ ] Create `.gitignore` for Rust and Node artifacts
- [ ] Task 2: Set up shared type system (AC: 4)
  - [ ] Define basic IPC envelope types in Rust (`types/src/lib.rs`)
  - [ ] Define matching TypeScript types in `types/src/index.ts`
  - [ ] Add `types` crate as dependency to `dyd-daemon`
  - [ ] Create build script to sync types if needed
- [ ] Task 3: Implement dev workflow (AC: 5)
  - [ ] Create `Makefile` with targets: `dev`, `build`, `lint`, `test`, `clean`
  - [ ] Install and configure `concurrently` for parallel process management
  - [ ] Wire `make dev` to start both `dyd-daemon` (cargo run) and `dyd-ui` (vite dev)
  - [ ] Add daemon stdout prefix `[daemon]` and UI prefix `[ui]`
  - [ ] Daemon hot-reload via `cargo watch` or equivalent
  - [ ] Vite HMR for frontend changes
- [ ] Task 4: Configure linting & formatting (AC: 6, 7, 8, 9)
  - [ ] Create `clippy.toml` with project-specific lint configuration
  - [ ] Add Rust lint scripts to `Makefile`
  - [ ] Install ESLint with flat config for TypeScript + React
  - [ ] Create `.prettierrc` with Tailwind plugin for class sorting
  - [ ] Add pre-commit hook configuration (optional)
- [ ] Task 5: Configure CI (AC: 10)
  - [ ] Create `.github/workflows/ci.yml`
  - [ ] Add Rust build, clippy, fmt --check, test steps
  - [ ] Add npm ci, lint, typecheck, build steps
  - [ ] Configure `Swatinem/rust-cache` and `actions/cache` for npm
  - [ ] Add status badge in README

## Dev Notes

- Root `Cargo.toml` virtual workspace — no `[package]`, only `[workspace]`
- `dyd-daemon` uses tokio async runtime, zbus for D-Bus, serde for serialization, tracing for logging
- `dyd-ui` uses Vite + React 18+ + TypeScript + Tailwind CSS v3+ + Motion (framer-motion)
- Types directory: Rust lib crate + TypeScript module, both defining the IPC protocol envelope
- Dev workflow: `concurrently` npm package spawns both processes with labeled output
- CI: GitHub Actions with proper caching for fast runs
- Use `cargo-watch` for Rust auto-recompilation on file changes
- ESLint flat config (`eslint.config.js`) not legacy `.eslintrc`

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
├── eslint.config.js
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

### Completion Notes List

### File List

## Change Log

- 2026-07-19: Initial story creation from Epic 1 Feature 1.1

## Status

Ready for implementation.
