use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::{broadcast, watch};
use tokio::time::{interval, Duration};

use dyd_types::{IpcEnvelope, IpcEvent, VolumePayload};

#[derive(Debug, Clone, PartialEq)]
struct VolumeState {
    volume: f64,
    muted: bool,
    sink_name: Option<String>,
}

impl Default for VolumeState {
    fn default() -> Self {
        Self {
            volume: 0.0,
            muted: false,
            sink_name: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum AudioTool {
    Wpctl,
    Pactl,
    None,
}

pub struct VolumeSource {
    tool: Arc<tokio::sync::RwLock<AudioTool>>,
    event_broadcast: broadcast::Sender<IpcEnvelope>,
    last: Arc<tokio::sync::RwLock<VolumeState>>,
}

impl VolumeSource {
    pub fn new() -> Self {
        let (event_broadcast, _) = broadcast::channel(32);
        Self {
            tool: Arc::new(tokio::sync::RwLock::new(AudioTool::None)),
            event_broadcast,
            last: Arc::new(tokio::sync::RwLock::new(VolumeState::default())),
        }
    }

    pub fn event_broadcast(&self) -> broadcast::Sender<IpcEnvelope> {
        self.event_broadcast.clone()
    }

    pub async fn start_polling(self: Arc<Self>, mut shutdown_rx: watch::Receiver<()>) {
        let mut tick = interval(Duration::from_millis(200));
        tick.tick().await;

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    tracing::info!("volume source shutting down");
                    break;
                }
                _ = tick.tick() => {
                    if let Err(e) = self.poll().await {
                        tracing::debug!(error = %e, "volume poll cycle error");
                    }
                }
            }
        }
    }

    async fn poll(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let tool = self.detect_tool().await?;
        match self.fetch_state(&tool).await {
            Ok(state) => {
                self.emit_if_changed(state).await;
                Ok(())
            }
            Err(e) => {
                *self.tool.write().await = AudioTool::None;
                tracing::debug!(error = %e, "volume fetch failed, clearing tool cache");
                Err(e)
            }
        }
    }

    async fn detect_tool(&self) -> Result<AudioTool, Box<dyn std::error::Error + Send + Sync>> {
        let mut cached = self.tool.write().await;
        if *cached != AudioTool::None {
            return Ok(cached.clone());
        }

        let wpctl_ok = run_quiet("wpctl", &["status"]).await.is_ok();
        if wpctl_ok {
            *cached = AudioTool::Wpctl;
            return Ok(AudioTool::Wpctl);
        }

        let pactl_ok = run_quiet("pactl", &["info"]).await.is_ok();
        if pactl_ok {
            *cached = AudioTool::Pactl;
            return Ok(AudioTool::Pactl);
        }

        Ok(AudioTool::None)
    }

    async fn fetch_state(
        &self,
        tool: &AudioTool,
    ) -> Result<VolumeState, Box<dyn std::error::Error + Send + Sync>> {
        match tool {
            AudioTool::None => Ok(VolumeState::default()),
            AudioTool::Wpctl => self.fetch_wpctl().await,
            AudioTool::Pactl => self.fetch_pactl().await,
        }
    }

    async fn fetch_wpctl(&self) -> Result<VolumeState, Box<dyn std::error::Error + Send + Sync>> {
        let output = Command::new("wpctl")
            .arg("get-volume")
            .arg("@DEFAULT_AUDIO_SINK@")
            .output()
            .await?;

        if !output.status.success() {
            return Err("wpctl get-volume failed".into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let muted = stdout.contains("[MUTED]");

        let volume = parse_fraction(&stdout).unwrap_or(0.0);

        Ok(VolumeState {
            volume,
            muted,
            sink_name: None,
        })
    }

    async fn fetch_pactl(&self) -> Result<VolumeState, Box<dyn std::error::Error + Send + Sync>> {
        let volume_out = Command::new("pactl")
            .arg("get-sink-volume")
            .arg("@DEFAULT_SINK@")
            .output()
            .await?;

        if !volume_out.status.success() {
            return Err("pactl get-sink-volume failed".into());
        }

        let volume_stdout = String::from_utf8_lossy(&volume_out.stdout).to_string();
        let volume = parse_pactl_percent(&volume_stdout).unwrap_or(0.0);

        let mute_out = Command::new("pactl")
            .arg("get-sink-mute")
            .arg("@DEFAULT_SINK@")
            .output()
            .await?;
        let muted = String::from_utf8_lossy(&mute_out.stdout).contains("yes");

        let sink_out = Command::new("pactl")
            .arg("get-default-sink")
            .output()
            .await?;
        let sink_name = if sink_out.status.success() {
            let name = String::from_utf8_lossy(&sink_out.stdout).trim().to_string();
            if name.is_empty() {
                None
            } else {
                Some(name)
            }
        } else {
            None
        };

        Ok(VolumeState {
            volume,
            muted,
            sink_name,
        })
    }

    async fn emit_if_changed(&self, state: VolumeState) {
        let mut last = self.last.write().await;
        if *last == state {
            return;
        }
        *last = state.clone();

        tracing::debug!(volume = %state.volume, muted = %state.muted, "volume changed");

        let payload = VolumePayload {
            volume: state.volume,
            muted: state.muted,
            sink_name: state.sink_name,
        };

        let envelope = IpcEnvelope::event(IpcEvent::VolumeChanged(payload));
        let _ = self.event_broadcast.send(envelope);
    }

    pub async fn set_volume(
        &self,
        volume: f64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let tool = self.detect_tool().await?;
        match tool {
            AudioTool::Wpctl => {
                let out = Command::new("wpctl")
                    .arg("set-volume")
                    .arg("@DEFAULT_AUDIO_SINK@")
                    .arg(volume.to_string())
                    .output()
                    .await?;
                if !out.status.success() {
                    return Err("wpctl set-volume failed".into());
                }
            }
            AudioTool::Pactl => {
                let percent = (volume.clamp(0.0, 1.0) * 100.0).round() as u32;
                let out = Command::new("pactl")
                    .arg("set-sink-volume")
                    .arg("@DEFAULT_SINK@")
                    .arg(format!("{percent}%"))
                    .output()
                    .await?;
                if !out.status.success() {
                    return Err("pactl set-sink-volume failed".into());
                }
            }
            AudioTool::None => return Err("no audio tool available".into()),
        }
        tracing::debug!(volume = %volume, "volume set");
        Ok(())
    }

    pub async fn set_mute(
        &self,
        _muted: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let tool = self.detect_tool().await?;
        match tool {
            AudioTool::Wpctl => {
                let out = Command::new("wpctl")
                    .arg("set-mute")
                    .arg("@DEFAULT_AUDIO_SINK@")
                    .arg("toggle")
                    .output()
                    .await?;
                if !out.status.success() {
                    return Err("wpctl set-mute failed".into());
                }
            }
            AudioTool::Pactl => {
                let out = Command::new("pactl")
                    .arg("set-sink-mute")
                    .arg("@DEFAULT_SINK@")
                    .arg("toggle")
                    .output()
                    .await?;
                if !out.status.success() {
                    return Err("pactl set-sink-mute failed".into());
                }
            }
            AudioTool::None => return Err("no audio tool available".into()),
        }
        tracing::debug!("volume mute toggled");
        Ok(())
    }
}

impl Default for VolumeSource {
    fn default() -> Self {
        Self::new()
    }
}

async fn run_quiet(
    program: &str,
    args: &[&str],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let status = Command::new(program).args(args).output().await?;
    if status.status.success() {
        Ok(())
    } else {
        Err("command exited non-zero".into())
    }
}

fn parse_fraction(output: &str) -> Option<f64> {
    output
        .find("Volume: ")
        .map(|i| &output[i + "Volume: ".len()..])
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|v| v.parse::<f64>().ok())
        .map(|v| v.clamp(0.0, 1.0))
}

fn parse_pactl_percent(output: &str) -> Option<f64> {
    output
        .find(" / ")
        .and_then(|i| {
            let rest = &output[i + 3..];
            let end = rest.find('%')?;
            rest[..end].trim().parse::<f64>().ok()
        })
        .map(|v| (v / 100.0).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let s = VolumeState::default();
        assert_eq!(s.volume, 0.0);
        assert!(!s.muted);
    }

    #[test]
    fn test_parse_wpctl_fraction() {
        assert_eq!(parse_fraction("Volume: 0.85"), Some(0.85));
        assert_eq!(parse_fraction("Volume: 0.85 [MUTED]"), Some(0.85));
        assert_eq!(parse_fraction("Volume: 1.00"), Some(1.0));
        assert_eq!(parse_fraction("nothing here"), None);
    }

    #[test]
    fn test_parse_pactl_percent() {
        let out = "Volume: front-left: 65536 / 100% / 0.00 dB, front-right: 65536 / 100% / 0.00 dB";
        assert_eq!(parse_pactl_percent(out), Some(1.0));
        let out2 = "Volume: front-left: 32768 / 50% / -18.06 dB";
        assert_eq!(parse_pactl_percent(out2), Some(0.5));
        assert_eq!(parse_pactl_percent("no data here"), None);
    }
}
