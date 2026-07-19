use tokio::signal::unix::{self, SignalKind};

pub fn sigterm_signal() -> tokio::signal::unix::Signal {
    unix::signal(SignalKind::terminate()).expect("Failed to create SIGTERM handler")
}

pub fn sigint_signal() -> tokio::signal::unix::Signal {
    unix::signal(SignalKind::interrupt()).expect("Failed to create SIGINT handler")
}
