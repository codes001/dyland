.PHONY: dev build build-release lint format format-check test clean deb appimage install uninstall

PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
DATADIR ?= $(PREFIX)/share

dev:
	npm run dev

build:
	cargo build --release && npm run build

build-release:
	cargo build --release
	cd dyd-ui && npm run build

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
	@mkdir -p dist/dyd_deb/DEBIAN dist/dyd_deb/usr/bin dist/dyd_deb/usr/share/applications dist/dyd_deb/usr/lib/systemd/user
	@printf "Package: dyland\nVersion: 0.1.0\nArchitecture: amd64\nMaintainer: dyland Maintainers <maintainers@dyland.org>\nDepends: libwebkit2gtk-4.1-0 | libwebkit2gtk-4.0-37, libgtk-3-0, libayatana-appindicator3-1\nSection: utils\nPriority: optional\nDescription: Dynamic Island for Linux Desktops\n Smooth, responsive overlay island bringing notifications, status, and media controls to Linux.\n" > dist/dyd_deb/DEBIAN/control
	@cp target/release/dyd-daemon dist/dyd_deb/usr/bin/dyd-daemon
	@cp resources/dyland.desktop dist/dyd_deb/usr/share/applications/dyland.desktop
	@cp resources/dyd-daemon.service dist/dyd_deb/usr/lib/systemd/user/dyd-daemon.service
	@dpkg-deb --build dist/dyd_deb dist/dyland_0.1.0_amd64.deb
	@echo "Created dist/dyland_0.1.0_amd64.deb"

appimage: build-release
	@bash scripts/build-appimage.sh

clean:
	cargo clean && rm -rf dist/

install: build-release
	install -Dm755 target/release/dyd-daemon $(DESTDIR)$(BINDIR)/dyd-daemon
	install -Dm644 resources/dyland.desktop $(DESTDIR)$(DATADIR)/applications/dyland.desktop
	install -Dm644 resources/dyd-daemon.service $(DESTDIR)$(DATADIR)/systemd/user/dyd-daemon.service
	@echo "dyland installed cleanly to $(PREFIX)."

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/dyd-daemon
	rm -f $(DESTDIR)$(DATADIR)/applications/dyland.desktop
	rm -f $(DESTDIR)$(DATADIR)/systemd/user/dyd-daemon.service
	@echo "dyland uninstalled."
