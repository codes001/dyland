export default function AboutSettings() {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <h3 style={{ fontSize: 14, margin: 0, color: "var(--pill-text)" }}>About dyland</h3>
      <p style={{ fontSize: 13, opacity: 0.9, margin: 0 }}>
        <strong>dyland</strong> — Dynamic Island for Linux Desktops
      </p>
      <p style={{ fontSize: 12, opacity: 0.7, margin: 0 }}>
        Version 0.1.0
      </p>
      <p style={{ fontSize: 12, opacity: 0.7, margin: 0 }}>
        Open source under the MIT License. Built with Rust, Tauri, React & WebAssembly.
      </p>
    </div>
  );
}
