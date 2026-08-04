//! `cargo-dyd` — build tooling for dyland WASM plugins.
//!
//! Invoked either directly (`cargo-dyd build`) or as a cargo subcommand
//! (`cargo dyd build`), in which case cargo passes `dyd` as the first argument.

use std::path::PathBuf;
use std::process::ExitCode;

use cargo_dyd::{build, install, package, plugins_dir, read_manifest, scaffold, DydError};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    bin_name = "cargo dyd",
    version,
    about = "Build, package, and install dyland WASM plugins"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scaffold a new plugin project.
    Init {
        /// Plugin name (also the crate name).
        name: String,
        /// Target directory (defaults to ./<name>).
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Build the plugin for wasm32-wasip1 (release).
    Build {
        /// Plugin project directory (defaults to cwd).
        #[arg(long, default_value = ".")]
        dir: PathBuf,
    },
    /// Package the plugin into a `.dyd` archive.
    Package {
        /// Plugin project directory (defaults to cwd).
        #[arg(long, default_value = ".")]
        dir: PathBuf,
        /// Output directory for the archive (defaults to <dir>/dist).
        #[arg(long)]
        out: Option<PathBuf>,
        /// Skip building and use the existing wasm artifact.
        #[arg(long)]
        no_build: bool,
    },
    /// Install a `.dyd` archive into the plugins directory.
    Install {
        /// Path to the `.dyd` archive.
        archive: PathBuf,
    },
}

fn run(cli: Cli) -> Result<(), DydError> {
    match cli.command {
        Command::Init { name, path } => {
            let dir = path.unwrap_or_else(|| PathBuf::from(&name));
            scaffold(&dir, &name)?;
            println!("Created plugin `{name}` in {}", dir.display());
            println!("Next: cd {} && cargo dyd build", dir.display());
            Ok(())
        }
        Command::Build { dir } => {
            let wasm = build(&dir)?;
            println!("Built {}", wasm.display());
            Ok(())
        }
        Command::Package { dir, out, no_build } => {
            let wasm = if no_build {
                cargo_dyd::find_wasm_artifact(&dir)?
            } else {
                build(&dir)?
            };
            let out_dir = out.unwrap_or_else(|| dir.join("dist"));
            let archive = package(&dir, &wasm, &out_dir)?;
            println!("Packaged {}", archive.display());
            Ok(())
        }
        Command::Install { archive } => {
            let root = plugins_dir();
            let dest = install(&archive, &root)?;
            let info = read_manifest(&dest).ok();
            match info {
                Some(i) => println!("Installed {} v{} to {}", i.name, i.version, dest.display()),
                None => println!("Installed to {}", dest.display()),
            }
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    // Support both `cargo-dyd <cmd>` and `cargo dyd <cmd>` (cargo injects the
    // `dyd` subcommand name as the first arg).
    let mut args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("dyd") {
        args.remove(1);
    }
    let cli = Cli::parse_from(args);
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
