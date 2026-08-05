//! Reference plugin: **VS Code git status**.
//!
//! Reads the focused VS Code window title (via GNOME Shell's D-Bus introspect
//! interface) and surfaces the repo, branch, dirty flag, and ahead/behind counts
//! as `[branch] +N` in the pill.
//!
//! ## Recommended VS Code setting (see README)
//! This plugin parses a title of the form:
//! ```text
//! ● myrepo (main ↑1↓2) — Visual Studio Code
//! ```
//! Configure VS Code's `window.title` to emit repo/branch/ahead-behind so the
//! parser has something to read. Titles without `(branch …)` parse as "No repo".
//!
//! ## Host-support caveat (see README)
//! The daemon's `dbus_call` host function (story 6-3) is a permission-gated
//! **stub** returning `-5 (unimplemented)`. Until real D-Bus wiring lands the
//! plugin can't fetch a live title, so it renders nothing (VS Code-not-running
//! path). The title parser below is fully unit-tested regardless.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};

use dyd_plugin_sdk::prelude::*;

const APP_SUFFIX: &str = "Visual Studio Code";

/// Ticks between title polls (daemon ticks at 100ms → ~1s).
const POLL_EVERY_TICKS: u32 = 10;

#[derive(Default)]
struct VsCodeGit {
    ticks: u32,
}

/// Git details extracted from a window title.
#[derive(Debug, Default, PartialEq)]
struct RepoStatus {
    repo: String,
    branch: String,
    dirty: bool,
    ahead: u32,
    behind: u32,
}

/// Result of parsing a window title.
#[derive(Debug, PartialEq)]
enum TitleParse {
    /// Title is not a VS Code window (no indicator should be shown).
    NotVsCode,
    /// VS Code is focused but no git repo is open.
    NoRepo,
    /// A repo/branch was found.
    Repo(RepoStatus),
}

/// Read the run of ASCII digits immediately following `marker` in `seg`.
fn count_after(seg: &str, marker: char) -> u32 {
    match seg.find(marker) {
        Some(idx) => {
            let after = &seg[idx + marker.len_utf8()..];
            let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse().unwrap_or(0)
        }
        None => 0,
    }
}

/// Parse `↑N` / `↓N` markers out of a branch segment, returning
/// `(branch_name, ahead, behind)`. Markers may be adjacent (`↑1↓2`).
fn parse_branch_segment(seg: &str) -> (String, u32, u32) {
    let ahead = count_after(seg, '↑');
    let behind = count_after(seg, '↓');
    // The branch name is whatever precedes the first arrow marker.
    let branch_end = seg.find(['↑', '↓']).unwrap_or(seg.len());
    let branch = seg[..branch_end].trim().to_string();
    (branch, ahead, behind)
}

/// Parse a VS Code window title into a [`TitleParse`].
fn parse_title(title: &str) -> TitleParse {
    if !title.contains(APP_SUFFIX) {
        return TitleParse::NotVsCode;
    }

    // Drop the trailing " — Visual Studio Code" (any dash variant / separator).
    let head = match title.rfind(APP_SUFFIX) {
        Some(idx) => &title[..idx],
        None => title,
    };
    let head = head.trim().trim_end_matches(['—', '-', '·', '|']).trim();

    // A leading dirty marker (● or *) means unstaged/uncommitted changes.
    let dirty = head.starts_with('●') || head.starts_with('*');
    let body = head.trim_start_matches(['●', '*']).trim();

    // Split "repo (branch ↑1↓2)" into repo + branch segment.
    if let (Some(open), Some(close)) = (body.find('('), body.rfind(')')) {
        if close > open {
            let repo = body[..open].trim().to_string();
            let seg = &body[open + 1..close];
            let (branch, ahead, behind) = parse_branch_segment(seg);
            if !branch.is_empty() {
                return TitleParse::Repo(RepoStatus {
                    repo,
                    branch,
                    dirty,
                    ahead,
                    behind,
                });
            }
        }
    }

    TitleParse::NoRepo
}

/// Build the pill/expanded activity for a repo status.
fn repo_activity(s: &RepoStatus) -> Value {
    let mut text = format!("[{}]", s.branch);
    if s.dirty {
        text.push_str(" ●");
    }
    if s.ahead > 0 {
        text.push_str(&format!(" ↑{}", s.ahead));
    }
    if s.behind > 0 {
        text.push_str(&format!(" ↓{}", s.behind));
    }
    json!({
        "text": text,
        "icon": "git-branch",
        "expanded": {
            "repo": s.repo,
            "branch": s.branch,
            "dirty": s.dirty,
            "ahead": s.ahead,
            "behind": s.behind,
        },
    })
}

impl Plugin for VsCodeGit {
    fn start(&mut self) {
        log_info!("vscode-git-status started");
    }

    fn tick(&mut self) {
        self.ticks += 1;
        if self.ticks < POLL_EVERY_TICKS {
            return;
        }
        self.ticks = 0;

        // GNOME Shell exposes focused-window info via Introspect. The reply
        // shape is compositor-specific; we ask for it and parse the title out.
        let title = match fetch_active_title() {
            Some(t) => t,
            None => return, // VS Code not running / D-Bus unavailable → no indicator.
        };

        match parse_title(&title) {
            TitleParse::NotVsCode => {}
            TitleParse::NoRepo => {
                emit_activity!(json!({ "text": "No repo", "icon": "git-branch" }));
            }
            TitleParse::Repo(status) => {
                emit_activity!(repo_activity(&status));
            }
        }
    }
}

/// Ask GNOME Shell for the focused window title. Returns `None` when the host
/// D-Bus bridge is unavailable (the current 6-3 stub) or no title is found.
fn fetch_active_title() -> Option<String> {
    let reply = dbus::call(
        "org.gnome.Shell",
        "/org/gnome/Shell/Introspect",
        "org.gnome.Shell.Introspect",
        "GetWindows",
    )
    .ok()?;
    // Expected shape: { "<id>": { "title": "...", "has-focus": true, ... }, ... }
    let windows = reply.as_object()?;
    for win in windows.values() {
        if win.get("has-focus").and_then(|v| v.as_bool()) == Some(true) {
            if let Some(title) = win.get("title").and_then(|v| v.as_str()) {
                return Some(title.to_string());
            }
        }
    }
    None
}

plugin_entry!(VsCodeGit);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_dirty_ahead_behind() {
        let t = "● myrepo (main ↑1↓2) — Visual Studio Code";
        match parse_title(t) {
            TitleParse::Repo(s) => {
                assert_eq!(s.repo, "myrepo");
                assert_eq!(s.branch, "main");
                assert!(s.dirty);
                assert_eq!(s.ahead, 1);
                assert_eq!(s.behind, 2);
            }
            other => panic!("expected Repo, got {other:?}"),
        }
    }

    #[test]
    fn clean_repo_has_no_dirty_flag() {
        let t = "project (develop) — Visual Studio Code";
        match parse_title(t) {
            TitleParse::Repo(s) => {
                assert_eq!(s.branch, "develop");
                assert!(!s.dirty);
                assert_eq!((s.ahead, s.behind), (0, 0));
            }
            other => panic!("expected Repo, got {other:?}"),
        }
    }

    #[test]
    fn non_vscode_title_is_ignored() {
        assert_eq!(parse_title("some-file.txt — gedit"), TitleParse::NotVsCode);
        assert_eq!(parse_title(""), TitleParse::NotVsCode);
    }

    #[test]
    fn vscode_without_branch_is_no_repo() {
        assert_eq!(
            parse_title("Welcome — Visual Studio Code"),
            TitleParse::NoRepo
        );
    }

    #[test]
    fn activity_formats_branch_and_markers() {
        let s = RepoStatus {
            repo: "r".to_string(),
            branch: "main".to_string(),
            dirty: true,
            ahead: 3,
            behind: 0,
        };
        let a = repo_activity(&s);
        assert_eq!(a["text"], "[main] ● ↑3");
        assert_eq!(a["expanded"]["behind"], 0);
    }
}
