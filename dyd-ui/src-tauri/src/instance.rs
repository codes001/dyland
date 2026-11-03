use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tokio::io::AsyncReadExt;

use crate::logging;
use crate::overlay;
use crate::paths;

pub enum Acquisition {
    Primary(Primary),
    Secondary,
}

pub struct Primary {
    _lock: File,
    socket_path: PathBuf,
}

impl Primary {
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }
}

impl Drop for Primary {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket_path);
        logging::info("released single-instance lock");
    }
}

pub fn acquire() -> std::io::Result<Acquisition> {
    let dir = paths::runtime_dir();
    fs::create_dir_all(&dir)?;
    let lock_path = dir.join("ui.lock");
    let socket_path = dir.join("ui.sock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)?;
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc != 0 {
        let err = std::io::Error::last_os_error();
        if is_contended(&err) {
            if let Err(notify_err) = notify_primary(&socket_path) {
                logging::error(&format!(
                    "dyland is already running, but the raise request failed: {notify_err}"
                ));
            }
            return Ok(Acquisition::Secondary);
        }
        return Err(err);
    }
    Ok(Acquisition::Primary(Primary {
        _lock: file,
        socket_path,
    }))
}

pub fn serve(socket_path: PathBuf, app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Err(err) = listen(socket_path, app).await {
            logging::error(&format!("instance listener stopped: {err}"));
        }
    });
}

async fn listen(socket_path: PathBuf, app: AppHandle) -> std::io::Result<()> {
    let _ = fs::remove_file(&socket_path);
    let listener = tokio::net::UnixListener::bind(&socket_path)?;
    fs::set_permissions(&socket_path, fs::Permissions::from_mode(0o600))?;
    logging::info(&format!("instance listener on {}", socket_path.display()));
    loop {
        let (mut stream, _) = listener.accept().await?;
        let mut buf = [0_u8; 32];
        let n = stream.read(&mut buf).await.unwrap_or(0);
        if parse_command(&buf[..n]) != Some(InstanceCommand::Raise) {
            continue;
        }
        logging::info("second launch requested the running island to come forward");
        let app = app.clone();
        let _ = app.clone().run_on_main_thread(move || {
            if let Some(window) = app.get_webview_window("main") {
                overlay::raise_existing(&window);
            }
        });
    }
}

fn notify_primary(socket_path: &Path) -> std::io::Result<()> {
    let mut last_error = std::io::Error::new(ErrorKind::NotFound, "instance socket is not ready");
    for _ in 0..20 {
        match UnixStream::connect(socket_path) {
            Ok(mut stream) => {
                stream.set_write_timeout(Some(Duration::from_secs(1)))?;
                stream.write_all(b"raise\n")?;
                return Ok(());
            }
            Err(err) => {
                last_error = err;
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
    Err(last_error)
}

fn is_contended(err: &std::io::Error) -> bool {
    matches!(
        err.raw_os_error(),
        Some(code) if code == libc::EWOULDBLOCK || code == libc::EAGAIN
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstanceCommand {
    Raise,
}

fn parse_command(bytes: &[u8]) -> Option<InstanceCommand> {
    let text = std::str::from_utf8(bytes).ok()?.trim();
    if text == "raise" {
        Some(InstanceCommand::Raise)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_raise_command() {
        assert_eq!(parse_command(b"raise\n"), Some(InstanceCommand::Raise));
        assert_eq!(parse_command(b" raise "), Some(InstanceCommand::Raise));
        assert_eq!(parse_command(b"quit\n"), None);
        assert_eq!(parse_command(b""), None);
    }
}
