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
	@mkdir -p dist/dyd_deb/DEBIAN dist/dyd_deb/usr/bin dist/dyd_deb/usr/share/applications dist/dyd_deb/usr/share/icons/hicolor/128x128/apps dist/dyd_deb/usr/lib/systemd/user dist/dyd_deb/etc/xdg/autostart dist/dyd_deb/usr/share/gnome-shell/extensions/dyland@dyland.app
	@printf "Package: dyland\nVersion: $(VERSION)\nArchitecture: amd64\nMaintainer: dyland Maintainers <maintainers@dyland.org>\nDepends: libwebkit2gtk-4.1-0 | libwebkit2gtk-4.0-37, libgtk-3-0, libayatana-appindicator3-1\nSection: utils\nPriority: optional\nDescription: Dynamic Island for Linux Desktops\n Smooth, responsive overlay island bringing notifications, status, and media controls to Linux.\n" > dist/dyd_deb/DEBIAN/control
	@cp target/release/dyd-daemon dist/dyd_deb/usr/bin/dyd-daemon
	@if [ -f target/release/dyd-ui-tauri ]; then cp target/release/dyd-ui-tauri dist/dyd_deb/usr/bin/dyd-ui; elif [ -f target/release/dyland ]; then cp target/release/dyland dist/dyd_deb/usr/bin/dyd-ui; fi
	@cp resources/dyland-session dist/dyd_deb/usr/bin/dyland-session
	@chmod 755 dist/dyd_deb/usr/bin/dyland-session
	@sed 's|@BINDIR@|/usr/bin|g' resources/dyland.desktop > dist/dyd_deb/usr/share/applications/dyland.desktop
	@sed 's|@BINDIR@|/usr/bin|g' resources/dyland-autostart.desktop > dist/dyd_deb/etc/xdg/autostart/dyland.desktop
	@cp dyd-ui/src-tauri/icons/128x128.png dist/dyd_deb/usr/share/icons/hicolor/128x128/apps/dyd.png
	@sed 's|@BINDIR@|/usr/bin|g' resources/dyd-daemon.service > dist/dyd_deb/usr/lib/systemd/user/dyd-daemon.service
	@sed 's|@BINDIR@|/usr/bin|g' resources/dyd-ui.service > dist/dyd_deb/usr/lib/systemd/user/dyd-ui.service
	@cp dyd-shell/metadata.json dist/dyd_deb/usr/share/gnome-shell/extensions/dyland@dyland.app/metadata.json
	@cp dyd-shell/extension.js dist/dyd_deb/usr/share/gnome-shell/extensions/dyland@dyland.app/extension.js
	@cp dyd-shell/stylesheet.css dist/dyd_deb/usr/share/gnome-shell/extensions/dyland@dyland.app/stylesheet.css
	@dpkg-deb --build dist/dyd_deb dist/dyland_$(VERSION)_amd64.deb
	@echo "Created dist/dyland_$(VERSION)_amd64.deb"

appimage: build-release
	@VERSION=$(VERSION) bash scripts/build-appimage.sh

clean:
	cargo clean && rm -rf dist/

install:
	@if [ ! -f target/release/dyd-daemon ] || [ ! -f target/release/dyd-ui-tauri ]; then \
		echo "Release binaries not found in target/release/."; \
		echo "Run 'make build-release' as your normal user, then 'sudo make install'."; \
		exit 1; \
	fi
	install -Dm755 target/release/dyd-daemon $(DESTDIR)$(BINDIR)/dyd-daemon
	install -Dm755 target/release/dyd-ui-tauri $(DESTDIR)$(BINDIR)/dyd-ui
	install -Dm755 resources/dyland-session $(DESTDIR)$(BINDIR)/dyland-session
	mkdir -p dist/install
	sed 's|@BINDIR@|$(BINDIR)|g' resources/dyland.desktop > dist/install/dyland.desktop
	sed 's|@BINDIR@|$(BINDIR)|g' resources/dyland-autostart.desktop > dist/install/dyland-autostart.desktop
	sed 's|@BINDIR@|$(BINDIR)|g' resources/dyd-daemon.service > dist/install/dyd-daemon.service
	sed 's|@BINDIR@|$(BINDIR)|g' resources/dyd-ui.service > dist/install/dyd-ui.service
	install -Dm644 dist/install/dyland.desktop $(DESTDIR)$(DATADIR)/applications/dyland.desktop
	install -Dm644 dist/install/dyland-autostart.desktop $(DESTDIR)/etc/xdg/autostart/dyland.desktop
	install -Dm644 dyd-ui/src-tauri/icons/128x128.png $(DESTDIR)$(DATADIR)/icons/hicolor/128x128/apps/dyd.png
	install -Dm644 dyd-shell/metadata.json $(DESTDIR)$(DATADIR)/gnome-shell/extensions/dyland@dyland.app/metadata.json
	install -Dm644 dyd-shell/extension.js $(DESTDIR)$(DATADIR)/gnome-shell/extensions/dyland@dyland.app/extension.js
	install -Dm644 dyd-shell/stylesheet.css $(DESTDIR)$(DATADIR)/gnome-shell/extensions/dyland@dyland.app/stylesheet.css
	install -Dm644 dist/install/dyd-daemon.service $(DESTDIR)$(DATADIR)/systemd/user/dyd-daemon.service
	install -Dm644 dist/install/dyd-ui.service $(DESTDIR)$(DATADIR)/systemd/user/dyd-ui.service
	@-which update-desktop-database >/dev/null 2>&1 && update-desktop-database $(DESTDIR)$(DATADIR)/applications || true
	@-which gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t $(DESTDIR)$(DATADIR)/icons/hicolor || true
	@echo "dyland installed to $(PREFIX)."
	@echo "Next graphical login starts it from /etc/xdg/autostart (no extra step)."
	@echo "To start it in this session now: make enable"

# Enable autostart for the current user and start the island now.
# XDG autostart from 'make install' already covers the next login.
enable:
	systemctl --user daemon-reload
	systemctl --user import-environment DISPLAY WAYLAND_DISPLAY XDG_RUNTIME_DIR DBUS_SESSION_BUS_ADDRESS XAUTHORITY XDG_CURRENT_DESKTOP || true
	systemctl --user reenable dyd-daemon.service dyd-ui.service
	systemctl --user start dyd-daemon.service dyd-ui.service
	rm -f $(HOME)/.config/autostart/dyland.desktop
	bash dyd-shell/install.sh
	@echo "dyland is running. It starts at every login until you run 'make disable'."

# Install the GNOME Shell extension that stacks the island above the top bar.
install-shell:
	bash dyd-shell/install.sh

disable:
	systemctl --user disable --now dyd-daemon.service dyd-ui.service || true
	mkdir -p $(HOME)/.config/autostart
	printf '%s\n' '[Desktop Entry]' 'Type=Application' 'Name=dyland' 'Hidden=true' > $(HOME)/.config/autostart/dyland.desktop
	-gnome-extensions disable dyland@dyland.app
	@echo "dyland will not start at login. The GNOME clock returns when the shell extension disables."

uninstall:
	systemctl --user disable --now dyd-daemon.service dyd-ui.service 2>/dev/null || true
	rm -f $(DESTDIR)$(BINDIR)/dyd-daemon
	rm -f $(DESTDIR)$(BINDIR)/dyd-ui
	rm -f $(DESTDIR)$(BINDIR)/dyland-session
	rm -f $(DESTDIR)$(DATADIR)/applications/dyland.desktop
	rm -f $(DESTDIR)/etc/xdg/autostart/dyland.desktop
	rm -f $(DESTDIR)$(DATADIR)/icons/hicolor/128x128/apps/dyd.png
	rm -f $(DESTDIR)$(DATADIR)/systemd/user/dyd-daemon.service
	rm -f $(DESTDIR)$(DATADIR)/systemd/user/dyd-ui.service
	rm -rf $(DESTDIR)$(DATADIR)/gnome-shell/extensions/dyland@dyland.app
	@echo "dyland uninstalled. Per-user autostart overrides remain in ~/.config/autostart until 'make disable'."
