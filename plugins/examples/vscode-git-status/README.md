# vscode-git-status

A reference [dyland](../../..) plugin that surfaces the focused VS Code window's
git status — `[branch] ● ↑1↓2` — in the Dynamic Island, with repo/branch/dirty
and ahead-behind detail in the expanded view.

## What it demonstrates

- Reading focused-window state through the daemon `dbus_call` host function.
- A pure, well-tested title parser with three outcomes: not-VS-Code, no-repo, repo.
- Graceful no-indicator behavior when VS Code isn't focused.

## Permissions

```json
"permissions": ["dbus:org.gnome.Shell.Introspect"]
```

## Recommended VS Code setting

The plugin parses a window title of the form:

```
● myrepo (main ↑1↓2) — Visual Studio Code
```

- leading `●` (or `*`) → dirty working tree
- `myrepo` → repo / root folder name
- `(main ↑1↓2)` → branch + commits ahead/behind

VS Code doesn't emit branch/ahead-behind in its title by default. Set
`window.title` (and a title-formatting extension if needed) so the title matches
the format above. Titles lacking a `(branch …)` segment parse as **"No repo"**.

## States

| Condition | Pill |
|---|---|
| Repo focused | `[main] ● ↑1↓2` |
| VS Code focused, no repo | `No repo` |
| VS Code not focused / not running | *(no indicator)* |

## ⚠️ Host-support caveat

The daemon's `dbus_call` host function (story 6-3) is a permission-gated **stub**
that returns `-5 (unimplemented)`. Until real D-Bus wiring lands, the plugin can't
fetch a live title and shows no indicator — it does **not** crash. The title
parser (the interesting part) is fully unit-tested and works unchanged once the
host bridges D-Bus.

## Build & test

```bash
cargo dyd build                              # release wasm32-wasip1 build
cargo test                                   # title-parser unit tests
cargo build --release --target wasm32-wasip1   # compile check
```
