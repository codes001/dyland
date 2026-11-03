use std::path::PathBuf;

pub fn graphical_session() -> bool {
    env_nonempty("WAYLAND_DISPLAY") || env_nonempty("DISPLAY")
}

pub fn runtime_dir() -> PathBuf {
    if let Some(dir) = env_path("XDG_RUNTIME_DIR") {
        return dir.join("dyland");
    }
    let uid = unsafe { libc::geteuid() };
    std::env::temp_dir().join(format!("dyland-{uid}"))
}

pub fn state_dir() -> PathBuf {
    if let Some(dir) = env_path("XDG_STATE_HOME") {
        return dir.join("dyland");
    }
    home_dir().join(".local/state/dyland")
}

fn env_nonempty(key: &str) -> bool {
    std::env::var(key)
        .map(|value| !value.is_empty())
        .unwrap_or(false)
}

fn env_path(key: &str) -> Option<PathBuf> {
    let value = std::env::var(key).ok()?;
    if value.is_empty() {
        None
    } else {
        Some(PathBuf::from(value))
    }
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .ok()
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_dir_uses_xdg_runtime_dir() {
        let previous = std::env::var("XDG_RUNTIME_DIR").ok();
        std::env::set_var("XDG_RUNTIME_DIR", "/run/user/1000");
        assert_eq!(runtime_dir(), PathBuf::from("/run/user/1000/dyland"));
        match previous {
            Some(value) => std::env::set_var("XDG_RUNTIME_DIR", value),
            None => std::env::remove_var("XDG_RUNTIME_DIR"),
        }
    }
}
