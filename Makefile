.PHONY: dev build lint format format-check test clean install uninstall

dev:
	npm run dev

build:
	cargo build --release && npm run build

lint:
	cargo clippy -- -D warnings
	npm run lint

format:
	cargo fmt && npm run format

format-check:
	cargo fmt --check && npm run format:check

test:
	cargo test

clean:
	cargo clean && npm run clean

install: build
	install -Dm755 target/release/dyd-daemon /usr/lib/dyd/dyd-daemon
	install -Dm644 dyd-daemon/systemd/dyd-daemon.service \
		/usr/lib/systemd/user/dyd-daemon.service
	systemctl --user daemon-reload
	systemctl --user enable --now dyd-daemon.service
	@echo "dyland installed. Run 'dyd-ui' to start the UI."

uninstall:
	systemctl --user stop dyd-daemon.service 2>/dev/null || true
	systemctl --user disable dyd-daemon.service 2>/dev/null || true
	rm -f /usr/lib/dyd/dyd-daemon
	rm -f /usr/lib/systemd/user/dyd-daemon.service
	systemctl --user daemon-reload
	@echo "dyland uninstalled."
