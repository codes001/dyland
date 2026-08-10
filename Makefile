.PHONY: dev build build-release lint format format-check test clean deb appimage install enable disable uninstall

PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
DATADIR ?= $(PREFIX)/share
VERSION ?= 0.1.0

dev:
	npm run dev

build:
	cargo build --release && npm run build

build-release:
	cargo build --release
	cd dyd-ui && npm run build
	cargo build --release --features custom-protocol --manifest-path dyd-ui/src-tauri/Cargo.toml

lint:
	cargo clippy -- -D warnings
	npm run lint

format:
	cargo fmt && npm run format

format-check:
	cargo fmt --check && npm run format:check

test:
	cargo test

deb: build-release
	@echo "Building Debian package..."
	@which dpkg-deb >/dev/null 2>&1 || (echo "dpkg-deb is required" && exit 1)
	@mkdir -p dist/dyd_deb/DEBIAN dist/dyd_deb/usr/bin dist/dyd_deb/usr/share/applications dist/dyd_deb/usr/share/icons/hicolor/128x128/apps dist/dyd_deb/usr/lib/systemd/user
	@printf "Package: dyland\nVersion: $(VERSION)\nArchitecture: amd64\nMaintainer: dyland Maintainers <maintainers@dyland.org>\nDepends: libwebkit2gtk-4.1-0 | libwebkit2gtk-4.0-37, libgtk-3-0, libayatana-appindicator3-1\nSection: utils\nPriority: optional\nDescription: Dynamic Island for Linux Desktops\n Smooth, responsive overlay island bringing notifications, status, and media controls to Linux.\n" > dist/dyd_deb/DEBIAN/control
	@cp target/release/dyd-daemon dist/dyd_deb/usr/bin/dyd-daemon
	@if [ -f target/release/dyd-ui-tauri ]; then cp target/release/dyd-ui-tauri dist/dyd_deb/usr/bin/dyd-ui; elif [ -f target/release/dyland ]; then cp target/release/dyland dist/dyd_deb/usr/bin/dyd-ui; fi
	@cp resources/dyland.desktop dist/dyd_deb/usr/share/applications/dyland.desktop
	@cp dyd-ui/src-tauri/icons/128x128.png dist/dyd_deb/usr/share/icons/hicolor/128x128/apps/dyd.png
	@cp resources/dyd-daemon.service dist/dyd_deb/usr/lib/systemd/user/dyd-daemon.service
	@cp resources/dyd-ui.service dist/dyd_deb/usr/lib/systemd/user/dyd-ui.service
	@dpkg-deb --build dist/dyd_deb dist/dyland_$(VERSION)_amd64.deb
	@echo "Created dist/dyland_$(VERSION)_amd64.deb"

appimage: build-release
	@VERSION=$(VERSION) bash scripts/build-appimage.sh

clean:
	cargo clean && rm -rf dist/

install:
	@if [ ! -f target/release/dyd-daemon ] || [ ! -f target/release/dyd-ui-tauri ]; then \
		echo "Release binaries not found in target/release/. Running build-release..."; \
		PATH="$$PATH:$$HOME/.cargo/bin:/root/.cargo/bin" cargo build --release || (echo "Failed to run cargo. Please run 'make build-release' as your normal user first, then run 'sudo make install'." && exit 1); \
		cd dyd-ui && npm run build && PATH="$$PATH:$$HOME/.cargo/bin:/root/.cargo/bin" cargo build --release --features custom-protocol --manifest-path src-tauri/Cargo.toml; \
	fi
	install -Dm755 target/release/dyd-daemon $(DESTDIR)$(BINDIR)/dyd-daemon
	install -Dm755 target/release/dyd-ui-tauri $(DESTDIR)$(BINDIR)/dyd-ui
	install -Dm644 resources/dyland.desktop $(DESTDIR)$(DATADIR)/applications/dyland.desktop
	install -Dm644 dyd-ui/src-tauri/icons/128x128.png $(DESTDIR)$(DATADIR)/icons/hicolor/128x128/apps/dyd.png
	install -Dm644 resources/dyd-daemon.service $(DESTDIR)$(DATADIR)/systemd/user/dyd-daemon.service
	install -Dm644 resources/dyd-ui.service $(DESTDIR)$(DATADIR)/systemd/user/dyd-ui.service
	@-which update-desktop-database >/dev/null 2>&1 && update-desktop-database $(DESTDIR)$(DATADIR)/applications || true
	@-which gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t $(DESTDIR)$(DATADIR)/icons/hicolor || true
	@echo "dyland installed cleanly to $(PREFIX)."
	@echo "Now enable autostart (as your normal user, NO sudo):"
	@echo "  make enable"

# Enable autostart for the current user: daemon + UI start at login,
# restart on crash, and survive reboots until disabled.
enable:
	systemctl --user daemon-reload
	systemctl --user enable --now dyd-daemon.service dyd-ui.service
	@echo "dyland enabled. It will now appear at every login until you run 'make disable'."

# Install the GNOME Shell extension that renders the island inside the shell
# (above every window, at the top of the screen). Requires a logout/login.
install-shell:
	bash dyd-shell/install.sh

disable:
	systemctl --user disable --now dyd-daemon.service dyd-ui.service
	@echo "dyland disabled."

uninstall:
	systemctl --user disable --now dyd-daemon.service dyd-ui.service 2>/dev/null || true
	rm -f $(DESTDIR)$(BINDIR)/dyd-daemon
	rm -f $(DESTDIR)$(BINDIR)/dyd-ui
	rm -f $(DESTDIR)$(DATADIR)/applications/dyland.desktop
	rm -f $(DESTDIR)$(DATADIR)/icons/hicolor/128x128/apps/dyd.png
	rm -f $(DESTDIR)$(DATADIR)/systemd/user/dyd-daemon.service
	rm -f $(DESTDIR)$(DATADIR)/systemd/user/dyd-ui.service
	@echo "dyland uninstalled."
