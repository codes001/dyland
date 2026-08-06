# Contributing Guide — dyland

Thank you for your interest in contributing to **dyland**!

---

## 🛠️ Development Setup

1. **Fork and Clone Repo**:
   ```bash
   git clone https://github.com/codes001/dyland.git
   cd dyland
   ```

2. **Install Dependencies**:
   - Follow the prerequisites in [INSTALL.md](INSTALL.md).
   - Install npm dependencies:
     ```bash
     cd dyd-ui && npm install
     ```

3. **Run Local Development**:
   - Start daemon:
     ```bash
     cargo run -p dyd-daemon
     ```
   - Start UI:
     ```bash
     cd dyd-ui && npm run dev
     ```

---

## 🧪 Testing & Linting

Before submitting a Pull Request, run all tests and linters:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test --workspace

cd dyd-ui
npm run typecheck
npm run build
```

---

## 📜 Pull Request Workflow

1. Create a descriptive feature branch (`git checkout -b feat/my-new-feature`).
2. Keep commits concise using Conventional Commits syntax (`feat:`, `fix:`, `docs:`, `refactor:`).
3. Ensure all CI checks pass on your branch.
4. Submit a Pull Request targeting `master`.
