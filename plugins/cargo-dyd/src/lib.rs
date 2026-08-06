//! Core logic for the `cargo-dyd` plugin build tool. Kept in a library so the
//! subcommands are unit-testable without spawning the binary.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub mod templates;

/// Error type for all cargo-dyd operations.
#[derive(Debug)]
pub enum DydError {
    Io(std::io::Error),
    Zip(String),
    Manifest(String),
    MissingTarget,
    Build(String),
    NotFound(String),
}

impl std::fmt::Display for DydError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DydError::Io(e) => write!(f, "io error: {e}"),
            DydError::Zip(e) => write!(f, "zip error: {e}"),
            DydError::Manifest(e) => write!(f, "manifest error: {e}"),
            DydError::MissingTarget => write!(
                f,
                "the wasm32-wasip1 target is not installed.\n\
                 Install it with:\n\n    rustup target add wasm32-wasip1\n"
            ),
            DydError::Build(e) => write!(f, "build failed: {e}"),
            DydError::NotFound(e) => write!(f, "not found: {e}"),
        }
    }
}

impl std::error::Error for DydError {}

impl From<std::io::Error> for DydError {
    fn from(e: std::io::Error) -> Self {
        DydError::Io(e)
    }
}

/// The subset of manifest fields cargo-dyd needs to name artifacts.
#[derive(Debug, serde::Deserialize)]
pub struct ManifestInfo {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub entry_point: Option<String>,
}

/// Read and minimally validate `manifest.json` in `dir`.
pub fn read_manifest(dir: &Path) -> Result<ManifestInfo, DydError> {
    let path = dir.join("manifest.json");
    let text = fs::read_to_string(&path)
        .map_err(|e| DydError::Manifest(format!("cannot read {}: {e}", path.display())))?;
    let info: ManifestInfo = serde_json::from_str(&text)
        .map_err(|e| DydError::Manifest(format!("invalid manifest.json: {e}")))?;
    if info.name.trim().is_empty() {
        return Err(DydError::Manifest("manifest `name` is empty".into()));
    }
    Ok(info)
}

// --- init ------------------------------------------------------------------

/// Scaffold a new plugin project at `dir` with the given crate/plugin name.
/// Writes `Cargo.toml`, `manifest.json`, and `src/lib.rs` from templates.
pub fn scaffold(dir: &Path, name: &str) -> Result<(), DydError> {
    if dir.exists() && fs::read_dir(dir)?.next().is_some() {
        return Err(DydError::Build(format!(
            "target directory {} is not empty",
            dir.display()
        )));
    }
    fs::create_dir_all(dir.join("src"))?;
    fs::write(dir.join("Cargo.toml"), templates::cargo_toml(name))?;
    fs::write(dir.join("manifest.json"), templates::manifest_json(name))?;
    fs::write(dir.join("src").join("lib.rs"), templates::lib_rs(name))?;
    Ok(())
}

// --- build -----------------------------------------------------------------

/// Whether the wasm32-wasip1 target is installed (via `rustup target list`).
pub fn wasm_target_installed() -> bool {
    std::process::Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("wasm32-wasip1"))
        .unwrap_or(false)
}

/// Build the plugin in `dir` for `wasm32-wasip1` (release). Returns the path to
/// the produced `.wasm` artifact. Emits [`DydError::MissingTarget`] with the
/// rustup hint if the target is absent.
pub fn build(dir: &Path) -> Result<PathBuf, DydError> {
    if !wasm_target_installed() {
        return Err(DydError::MissingTarget);
    }
    let status = std::process::Command::new("cargo")
        .current_dir(dir)
        .args(["build", "--release", "--target", "wasm32-wasip1"])
        .status()
        .map_err(|e| DydError::Build(format!("failed to run cargo: {e}")))?;
    if !status.success() {
        return Err(DydError::Build("cargo build returned non-zero".into()));
    }
    find_wasm_artifact(dir)
}

/// Locate the built `.wasm` under `target/wasm32-wasip1/release/`.
pub fn find_wasm_artifact(dir: &Path) -> Result<PathBuf, DydError> {
    let release = dir.join("target").join("wasm32-wasip1").join("release");
    if !release.exists() {
        return Err(DydError::NotFound(format!(
            "no build output at {}",
            release.display()
        )));
    }
    for entry in fs::read_dir(&release)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) == Some("wasm") {
            return Ok(path);
        }
    }
    Err(DydError::NotFound(format!(
        "no .wasm artifact in {}",
        release.display()
    )))
}

// --- package ---------------------------------------------------------------

/// Create a `{name}-{version}.dyd` zip in `out_dir` containing `manifest.json`,
/// `plugin.wasm` (from `wasm_path`), and any files under `dir/assets/`.
/// Returns the path to the written archive.
pub fn package(dir: &Path, wasm_path: &Path, out_dir: &Path) -> Result<PathBuf, DydError> {
    let info = read_manifest(dir)?;
    fs::create_dir_all(out_dir)?;
    let archive_path = out_dir.join(format!("{}-{}.dyd", info.name, info.version));
    let file = fs::File::create(&archive_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let opts: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // manifest.json
    let manifest = fs::read(dir.join("manifest.json"))?;
    zip.start_file("manifest.json", opts)
        .map_err(|e| DydError::Zip(e.to_string()))?;
    zip.write_all(&manifest)?;

    // plugin.wasm
    let wasm = fs::read(wasm_path)?;
    zip.start_file("plugin.wasm", opts)
        .map_err(|e| DydError::Zip(e.to_string()))?;
    zip.write_all(&wasm)?;

    // optional assets/
    let assets = dir.join("assets");
    if assets.is_dir() {
        add_dir_to_zip(&mut zip, &assets, "assets", opts)?;
    }

    zip.finish().map_err(|e| DydError::Zip(e.to_string()))?;
    Ok(archive_path)
}

fn add_dir_to_zip<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    dir: &Path,
    prefix: &str,
    opts: zip::write::FileOptions<'_, ()>,
) -> Result<(), DydError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let zip_path = format!("{prefix}/{name}");
        if path.is_dir() {
            add_dir_to_zip(zip, &path, &zip_path, opts)?;
        } else {
            zip.start_file(zip_path, opts)
                .map_err(|e| DydError::Zip(e.to_string()))?;
            let bytes = fs::read(&path)?;
            zip.write_all(&bytes)?;
        }
    }
    Ok(())
}

// --- install ---------------------------------------------------------------

/// Resolve the plugins install directory: `$XDG_DATA_HOME/dyd/plugins` (falling
/// back to `$HOME/.local/share/dyd/plugins`). `DYD_PLUGINS_DIR` overrides both,
/// matching the daemon's resolution.
pub fn plugins_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("DYD_PLUGINS_DIR") {
        return PathBuf::from(dir);
    }
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".local").join("share")
        });
    base.join("dyd").join("plugins")
}

/// Install a `.dyd` archive by unzipping it to `{plugins_dir}/{name}/`. Returns
/// the install directory. `name` is taken from the bundled `manifest.json`.
pub fn install(archive: &Path, plugins_root: &Path) -> Result<PathBuf, DydError> {
    let file = fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| DydError::Zip(e.to_string()))?;

    // Read the manifest first to learn the plugin name.
    let name = {
        let mut mf = zip
            .by_name("manifest.json")
            .map_err(|_| DydError::Manifest("archive has no manifest.json".into()))?;
        let mut text = String::new();
        mf.read_to_string(&mut text)?;
        let info: ManifestInfo = serde_json::from_str(&text)
            .map_err(|e| DydError::Manifest(format!("invalid bundled manifest.json: {e}")))?;
        info.name
    };

    let dest = plugins_root.join(&name);
    if dest.exists() {
        fs::remove_dir_all(&dest)?;
    }
    fs::create_dir_all(&dest)?;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| DydError::Zip(e.to_string()))?;
        let rel = match entry.enclosed_name() {
            Some(p) => p.to_path_buf(),
            None => continue, // reject path-traversal entries
        };
        let out_path = dest.join(&rel);
        if entry.is_dir() {
            fs::create_dir_all(&out_path)?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(&out_path)?;
        std::io::copy(&mut entry, &mut out)?;
    }
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_manifest(dir: &Path, name: &str, version: &str) {
        fs::create_dir_all(dir).unwrap();
        let mf = serde_json::json!({
            "schema_version": 1,
            "name": name,
            "version": version,
            "author": "t",
            "entry_point": "plugin.wasm"
        });
        fs::write(dir.join("manifest.json"), mf.to_string()).unwrap();
    }

    #[test]
    fn scaffold_creates_expected_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("myplugin");
        scaffold(&dir, "my-plugin").unwrap();
        assert!(dir.join("manifest.json").exists());
        assert!(dir.join("Cargo.toml").exists());
        assert!(dir.join("src").join("lib.rs").exists());
        // manifest names the plugin.
        let info = read_manifest(&dir).unwrap();
        assert_eq!(info.name, "my-plugin");
        // lib.rs uses the SDK entry macro.
        let lib = fs::read_to_string(dir.join("src").join("lib.rs")).unwrap();
        assert!(lib.contains("plugin_entry!"));
    }

    #[test]
    fn scaffold_refuses_nonempty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("x"), "y").unwrap();
        let err = scaffold(tmp.path(), "p").unwrap_err();
        assert!(matches!(err, DydError::Build(_)));
    }

    #[test]
    fn package_zip_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("plugin");
        write_manifest(&src, "demo", "1.2.3");
        // assets/
        fs::create_dir_all(src.join("assets")).unwrap();
        fs::write(src.join("assets").join("icon.txt"), "icon").unwrap();
        // fake wasm artifact
        let wasm = tmp.path().join("plugin.wasm");
        fs::write(&wasm, b"\0asm fake").unwrap();

        let out = tmp.path().join("dist");
        let archive = package(&src, &wasm, &out).unwrap();
        assert_eq!(archive.file_name().unwrap(), "demo-1.2.3.dyd");

        // Inspect the archive contents.
        let f = fs::File::open(&archive).unwrap();
        let mut zip = zip::ZipArchive::new(f).unwrap();
        let names: Vec<String> = (0..zip.len())
            .map(|i| zip.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(names.contains(&"manifest.json".to_string()));
        assert!(names.contains(&"plugin.wasm".to_string()));
        assert!(names.contains(&"assets/icon.txt".to_string()));
    }

    #[test]
    fn package_then_install_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("plugin");
        write_manifest(&src, "roundtrip", "0.1.0");
        let wasm = tmp.path().join("plugin.wasm");
        fs::write(&wasm, b"\0asm").unwrap();
        let out = tmp.path().join("dist");
        let archive = package(&src, &wasm, &out).unwrap();

        let root = tmp.path().join("install-root");
        let dest = install(&archive, &root).unwrap();
        assert_eq!(dest, root.join("roundtrip"));
        assert!(dest.join("manifest.json").exists());
        assert!(dest.join("plugin.wasm").exists());
    }

    #[test]
    fn missing_target_error_mentions_rustup() {
        let msg = DydError::MissingTarget.to_string();
        assert!(msg.contains("rustup target add wasm32-wasip1"));
    }

    #[test]
    fn plugins_dir_respects_override() {
        std::env::set_var("DYD_PLUGINS_DIR", "/tmp/dyd-override-xyz");
        assert_eq!(plugins_dir(), PathBuf::from("/tmp/dyd-override-xyz"));
        std::env::remove_var("DYD_PLUGINS_DIR");
    }
}
