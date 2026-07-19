# Epic 9: Distribution

## Objective
Package the application for distribution to Ubuntu 24.04 LTS and other Linux distributions. Provide .deb packages, AppImage builds, CI/CD pipeline, auto-update mechanism, and user-facing documentation.

## Business Value
A well-distributed application reaches users. Packaging, updates, and documentation are essential for adoption. Without this Epic, the application remains a developer-only tool.

## Success Criteria
- .deb package installable on Ubuntu 24.04 LTS
- AppImage build for distribution-agnostic installation
- CI/CD pipeline builds and tests on every commit, publishes on tag
- Auto-update mechanism checks for and applies updates
- User documentation covers installation, configuration, and usage
- `make install` / `make uninstall` works for development

## Risks
- Tauri's deb bundling may have quirks on different Ubuntu versions
- Auto-update on Linux is fragmented (no standard mechanism like macOS Sparkle)
- AppImage requires FUSE which may not be available by default on Ubuntu 24.04
- Documentation must be maintained alongside code changes

## Dependencies
- All previous Epics (distribution packages the complete application)

## Complexity
⭐⭐ — Low complexity. Well-understood packaging patterns for Linux.

---

## Feature 9.1: .deb Package

**Description:** Create a Debian package (.deb) for installation on Ubuntu 24.04 LTS and Debian-based distributions. The package installs the daemon binary, UI resources, systemd service, and desktop entry.

**Acceptance Criteria:** `dpkg -i dyd.deb` installs the application. Application appears in GNOME application grid. Systemd service enabled. `dpkg -r dyd` removes cleanly.

**Dependencies:** 1.6 (systemd service)

**Notes:** Ubuntu 24.04 is the primary target. Debian compatibility is secondary. Use Tauri's built-in bundler for the UI component.

### User Stories

#### US-9.1.1 — Debian Package Build
**As a** developer
**I want** to build a .deb package from the project
**so that** users can install on Debian-based distributions.

**Acceptance Criteria:**
- `make deb` or `cargo xtask deb` produces `dyd-{version}-amd64.deb`
- Package contents:
  - `/usr/lib/dyd/dyd-daemon` — daemon binary
  - `/usr/lib/dyd/dyd-ui` — Tauri webview resources
  - `/usr/lib/systemd/user/dyd-daemon.service` — systemd user service
  - `/usr/share/applications/dyd.desktop` — desktop entry
  - `/usr/share/icons/hicolor/{size}/apps/dyd.png` — application icons
  - `/usr/share/dyd/` — bundled assets
- Package dependencies: `libwebkit2gtk-4.1`, `libgtk-3-0`, `libayatana-appindicator3-1`, `libjavascriptcoregtk-4.1`

**Edge Cases:**
- Architecture-specific build (amd64 only for v1)
- Missing dependencies → package manager handles gracefully

**Technical Notes:**
- Tauri's `tauri build --bundles deb` handles UI bundling
- Custom `after-install` script to `systemctl --user daemon-reload` and `enable`
- Control file with proper dependencies
- Use `dpkg-deb` for final packaging

**Priority:** P1

#### US-9.1.2 — Desktop Entry & Icons
**As an** end user
**I want** the application to appear in my GNOME application grid with an icon
**so that** I can launch it like any other application.

**Acceptance Criteria:**
- `.desktop` file installed to `/usr/share/applications/dyd.desktop`
- Desktop file specifies: name "dyland", icon, categories "Utility", comment
- Application icon in multiple sizes: 48×48, 64×64, 128×128, 256×256
- Icon follows Tauri icon guidelines (PNG with alpha)
- Running the desktop entry launches both daemon and UI

**Edge Cases:**
- Icon not found → GNOME shows generic icon
- Running without daemon installed → desktop entry could fail gracefully

**Technical Notes:**
- Desktop file:
  ```
  [Desktop Entry]
  Name=dyland
  Comment=Dynamic Island for Linux
  Exec=/usr/lib/dyd/dyd-ui
  Type=Application
  Categories=Utility;
  Icon=dyd
  Terminal=false
  ```
- Icon generation: `cargo tauri icon` from a 1024×1024 source PNG

**Priority:** P1

---

## Feature 9.2: AppImage Build

**Description:** Create an AppImage build for distribution-agnostic installation on any Linux distribution without requiring package manager integration.

**Acceptance Criteria:** AppImage file runs on Ubuntu 24.04, Fedora 40, and Arch Linux. AppImage is self-contained (all dependencies bundled). AppImage can be run from anywhere (no installation needed).

**Dependencies:** 9.1

**Notes:** AppImage is the easiest distribution method for users who don't use Debian-based distros. FUSE may need to be installed separately.

### User Stories

#### US-9.2.1 — AppImage Build Configuration
**As a** developer
**I want** to produce an AppImage from the project
**so that** users on any Linux distribution can run the application.

**Acceptance Criteria:**
- `make appimage` produces `dyd-{version}-x86_64.AppImage`
- AppImage bundles: daemon binary, UI webview resources, systemd service file (not auto-installed), all shared library dependencies
- AppImage runs on: Ubuntu 24.04, Fedora 40, Arch Linux (without additional packages)
- AppImage sets up systemd user service on first run (optional, prompt user)

**Edge Cases:**
- FUSE not available → AppImage provides instructions or alternative extraction
- libfuse2 vs libfuse3 → AppImage should work with either

**Technical Notes:**
- Use `appimagetool` from AppImageKit
- Tauri doesn't natively support AppImage → custom build step
- AppDir structure:
  ```
  dyd.AppDir/
  ├── AppRun (launch script)
  ├── dyd-daemon
  ├── dyd-ui
  ├── usr/
  │   ├── lib/ (bundled libraries)
  │   └── share/ (icons, desktop file)
  └── dyd.desktop
  ```

**Priority:** P2

---

## Feature 9.3: CI/CD Pipeline

**Description:** Set up GitHub Actions CI/CD pipeline that builds, lints, tests, and packages the application on every commit, and publishes releases on version tags.

**Acceptance Criteria:** CI runs on every PR (build + lint + test). CD builds .deb and AppImage on tag push. Release artifacts are published to GitHub Releases. Pipeline completes in under 10 minutes.

**Dependencies:** 1.1.4 (CI configuration)

**Notes:** Use GitHub Actions for CI/CD. Matrix build for different features (debug/release). Artifact caching for faster builds.

### User Stories

#### US-9.3.1 — CI Pipeline
**As a** developer
**I want** continuous integration that runs on every commit
**so that** regressions are caught immediately.

**Acceptance Criteria:**
- Workflow triggers: push to any branch, PR to main
- Steps:
  1. Checkout code
  2. Install system dependencies (libwebkit2gtk, etc.)
  3. Cache Rust and npm dependencies
  4. `cargo build` (debug)
  5. `cargo clippy` (no warnings)
  6. `cargo fmt --check`
  7. `cargo test`
  8. `npm ci`
  9. `npm run lint`
  10. `npm run typecheck`
  11. `npm run build`
- Pipeline completes in under 10 minutes
- Status badges in README

**Edge Cases:**
- Flaky tests → retry mechanism (max 2 retries)
- Dependency cache miss → full build (slower but correct)

**Technical Notes:**
- Use `actions-rs/toolchain` for Rust toolchain
- Use `Swatinem/rust-cache` for Rust dependency caching
- Use `actions/setup-node` and `actions/cache` for npm

**Priority:** P1

#### US-9.3.2 — CD Pipeline & Release
**As a** developer
**I want** automatic release builds when I push a version tag
**so that** I can publish new versions without manual steps.

**Acceptance Criteria:**
- Trigger: push tag matching `v*.*.*` (e.g., `v0.1.0`)
- Build steps (release mode):
  1. `cargo build --release`
  2. `npm run build`
  3. `tauri build --bundles deb`
  4. `make appimage`
- Artifacts: `.deb` package, `.AppImage`, checksums (SHA256)
- GitHub Release created with:
  - Auto-generated release notes from conventional commits
  - Artifacts attached
  - `CHANGELOG.md` updated (if using commit-and-tag workflow)
- Version number extracted from `Cargo.toml`

**Edge Cases:**
- Tag without matching version in Cargo.toml → error
- Existing release → skip (or fail)

**Technical Notes:**
- Use `softprops/action-gh-release` for GitHub Release creation
- Release notes from `git log --oneline` between tags, or use `git-cliff`
- Sign artifacts with GPG (optional, future)

**Priority:** P2

---

## Feature 9.4: Auto-Update Mechanism

**Description:** Implement an auto-update mechanism that checks for new versions and applies updates. On Linux, this typically means downloading and replacing the binary/package.

**Acceptance Criteria:** Application checks for updates on startup and periodically (every 24h). New version notification appears in Dynamic Island. User can download and apply update from the notification. Update applies on next restart.

**Dependencies:** 9.3 (release artifacts)

**Notes:** Tauri has a built-in updater plugin (`tauri-plugin-updater`). Use it for the UI side. Daemon updates are trickier — may require a separate mechanism.

### User Stories

#### US-9.4.1 — Update Check
**As a** system
**I want** to periodically check for new versions
**so that** the user knows when an update is available.

**Acceptance Criteria:**
- Check on startup and every 24 hours
- Check: GET `https://github.com/{owner}/{repo}/releases/latest` API
- Compare latest version against current version (semver)
- If newer version found:
  - Store notification in activity queue
  - Show: "Update available: v{new_version}"
  - Download size estimate displayed
- If same or error: silent (log only)

**Edge Cases:**
- No network → skip check, retry on next startup
- Rate limited by GitHub API → cache last check result for 24h
- Pre-release version → include in check? Configurable

**Technical Notes:**
- Use `reqwest` with a timeout (10s)
- Version comparison: `semver` crate
- IPC event: `UpdateAvailable { version, url, size }`
- Check only in daemon (UI has no network access by default in Tauri)

**Priority:** P2

#### US-9.4.2 — Update Application
**As an** end user
**I want** to download and install updates from the Dynamic Island
**so that** I stay up-to-date without manual intervention.

**Acceptance Criteria:**
- Click "Update" in update notification → download starts
- Download progress shown (percentage or indeterminate)
- After download:
  - .deb: prompt to install with pkexec (`pkexec dpkg -i`)
  - AppImage: download new AppImage, replace old one
- After install: prompt to restart (or auto-restart)
- Rollback: if update fails, keep old version

**Edge Cases:**
- Permission denied → show instructions for manual update
- Download interrupted → retry on next check
- Update while daemon is in critical state (incoming call) → defer

**Technical Notes:**
- Download to `/tmp/dyd-update/`
- Verify checksum before installing
- For .deb: use `pkexec dpkg -i` (PolicyKit prompt)
- For AppImage: replace file (simpler, no root needed)

**Priority:** P3

---

## Feature 9.5: User Documentation

**Description:** Create user-facing documentation covering installation, configuration, usage, and troubleshooting. Documentation is versioned and published alongside releases.

**Acceptance Criteria:** README updated with installation instructions. User guide covers all features. FAQ section. Troubleshooting guide. Documentation is in the repository and published to GitHub Pages (optional).

**Dependencies:** All Epics (documents the final product)

**Notes:** Documentation is as important as code. A well-documented application has higher adoption and lower support burden.

### User Stories

#### US-9.5.1 — Installation Guide
**As an** end user
**I want** clear installation instructions for Ubuntu 24.04 and other distributions
**so that** I can get the application running quickly.

**Acceptance Criteria:**
- README.md contains:
  - Prerequisites (Ubuntu 24.04, GNOME 46, Wayland)
  - Installation methods: .deb package, AppImage, building from source
  - Step-by-step instructions for each method
  - Verification step: how to confirm the app is running
  - Uninstallation instructions
- Screenshots/GIFs of the running application

**Edge Cases:**
- User on unsupported distribution → documented as "community-supported", no guarantees

**Technical Notes:**
- README in repo root
- Separate `INSTALL.md` for detailed installation guide

**Priority:** P1

#### US-9.5.2 — User Guide
**As an** end user
**I want** to understand how to use the Dynamic Island
**so that** I can take advantage of all its features.

**Acceptance Criteria:**
- User guide covers:
  - Basic usage: what the pill shows, how to interact
  - Gestures: click, swipe, long-press
  - Media controls: how to play/pause/skip
  - Notifications: how they appear, actions, reply
  - System status: battery, WiFi, volume, brightness
  - Settings: how to access and configure
  - DND mode: how to enable/disable/schedule
  - Plugins: how to install and manage
  - Keyboard shortcuts
- Screenshots for each feature
- Written in clear, user-friendly language

**Edge Cases:**
- Guide should be discoverable from the application (link in About settings)

**Technical Notes:**
- File: `docs/user-guide.md`
- Consider using mdbook for a nicer documentation site

**Priority:** P2

#### US-9.5.3 — Developer Guide
**As a** developer
**I want** to understand the architecture and how to contribute
**so that** I can build plugins or contribute to the core.

**Acceptance Criteria:**
- Developer guide covers:
  - Architecture overview (dual-process, IPC)
  - Project structure
  - Development setup
  - Building and testing
  - Plugin API documentation
  - How to contribute (PR workflow, coding standards)
  - Architecture Decision Records (ADRs) for major decisions

**Edge Cases:**
- Outdated documentation → CI check for doc freshness (future)

**Technical Notes:**
- File: `ARCHITECTURE.md` and `CONTRIBUTING.md`
- ADRs in `docs/adr/` directory

**Priority:** P2
