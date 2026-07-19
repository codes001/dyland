#!/usr/bin/env python3
"""Generate an HTML evaluation review viewer."""

import argparse
import base64
import html
import json
import os
import sys
from pathlib import Path


def load_json(path: str) -> dict | None:
    if not os.path.exists(path):
        return None
    try:
        with open(path, "r") as f:
            return json.load(f)
    except (json.JSONDecodeError, IOError):
        return None


def read_output_files(outputs_dir: str) -> dict[str, str]:
    """Read all files in an outputs directory."""
    files = {}
    if not os.path.isdir(outputs_dir):
        return files
    for fname in sorted(os.listdir(outputs_dir)):
        fpath = os.path.join(outputs_dir, fname)
        if os.path.isfile(fpath):
            try:
                with open(fpath, "r", encoding="utf-8", errors="replace") as f:
                    files[fname] = f.read()
            except Exception:
                files[fname] = "[Binary file]"
    return files


def generate_html(workspace: str, skill_name: str, benchmark: dict, previous_workspace: str | None = None) -> str:
    """Generate the review HTML."""

    iteration_dir = os.path.basename(os.path.normpath(workspace))

    # Collect eval data
    evals_data = []
    if os.path.isdir(workspace):
        for item in sorted(os.listdir(workspace)):
            item_path = os.path.join(workspace, item)
            if not os.path.isdir(item_path) or not item.startswith("eval-"):
                continue

            metadata = load_json(os.path.join(item_path, "eval_metadata.json")) or {}

            # Current outputs
            with_skill_files = read_output_files(os.path.join(item_path, "with_skill", "outputs"))
            baseline_files = {}
            for baseline in ["without_skill", "old_skill"]:
                baseline_dir = os.path.join(item_path, baseline, "outputs")
                if os.path.isdir(baseline_dir):
                    baseline_files = read_output_files(baseline_dir)
                    baseline_label = baseline
                    break
            else:
                baseline_label = "baseline"

            # Previous outputs
            prev_with_skill_files = {}
            if previous_workspace:
                prev_path = os.path.join(previous_workspace, item, "with_skill", "outputs")
                prev_with_skill_files = read_output_files(prev_path)

            # Grading
            with_skill_grading = load_json(os.path.join(item_path, "with_skill", "grading.json")) or {"expectations": []}
            baseline_grading = load_json(os.path.join(item_path, baseline_label, "grading.json")) or {"expectations": []}

            # Timing
            with_skill_timing = load_json(os.path.join(item_path, "with_skill", "timing.json")) or {}
            baseline_timing = load_json(os.path.join(item_path, baseline_label, "timing.json")) or {}

            evals_data.append({
                "name": item,
                "eval_name": metadata.get("eval_name", item),
                "prompt": metadata.get("prompt", "No prompt recorded"),
                "with_skill_files": with_skill_files,
                "baseline_files": baseline_files,
                "baseline_label": baseline_label,
                "prev_with_skill_files": prev_with_skill_files,
                "with_skill_grading": with_skill_grading,
                "baseline_grading": baseline_grading,
                "with_skill_timing": with_skill_timing,
                "baseline_timing": baseline_timing,
            })

    # Benchmark data for JS
    benchmark_json = json.dumps(benchmark or {})
    evals_json = json.dumps(evals_data)

    html_template = f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Skill Eval Review: {html.escape(skill_name)}</title>
<style>
  :root {{
    --bg: #0d1117;
    --surface: #161b22;
    --border: #30363d;
    --text: #c9d1d9;
    --text-secondary: #8b949e;
    --accent: #58a6ff;
    --success: #3fb950;
    --danger: #f85149;
    --warning: #d29922;
  }}
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
    background: var(--bg);
    color: var(--text);
    line-height: 1.5;
    height: 100vh;
    display: flex;
    flex-direction: column;
  }}
  header {{
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    padding: 12px 24px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-shrink: 0;
  }}
  header h1 {{ font-size: 16px; font-weight: 600; }}
  header .meta {{ color: var(--text-secondary); font-size: 13px; }}
  .tabs {{
    display: flex;
    gap: 4px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    padding: 0 24px;
    flex-shrink: 0;
  }}
  .tab {{
    padding: 10px 16px;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    font-size: 14px;
    color: var(--text-secondary);
    transition: all 0.15s;
  }}
  .tab:hover {{ color: var(--text); }}
  .tab.active {{ color: var(--accent); border-bottom-color: var(--accent); }}
  .content {{ flex: 1; overflow: auto; padding: 24px; }}
  .tab-panel {{ display: none; }}
  .tab-panel.active {{ display: block; }}

  .eval-nav {{
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--border);
  }}
  .eval-nav button {{
    background: var(--surface);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 6px 14px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
  }}
  .eval-nav button:hover {{ border-color: var(--accent); }}
  .eval-nav button:disabled {{ opacity: 0.4; cursor: not-allowed; }}
  .eval-counter {{ font-size: 14px; color: var(--text-secondary); }}

  .section {{ margin-bottom: 24px; }}
  .section-title {{
    font-size: 13px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }}
  .prompt-box {{
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px;
    font-size: 14px;
    white-space: pre-wrap;
    word-break: break-word;
  }}
  .file-box {{
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    margin-bottom: 12px;
    overflow: hidden;
  }}
  .file-header {{
    background: rgba(48, 54, 61, 0.5);
    padding: 8px 16px;
    font-size: 12px;
    font-weight: 600;
    border-bottom: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
  }}
  .file-content {{
    padding: 16px;
    font-size: 13px;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 500px;
    overflow: auto;
    font-family: "SFMono-Regular", Consolas, monospace;
  }}
  .comparison {{
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }}
  .comparison-col {{
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px;
  }}
  .comparison-col h3 {{
    font-size: 13px;
    font-weight: 600;
    margin-bottom: 12px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border);
  }}
  .grading-item {{
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
    font-size: 13px;
  }}
  .grading-item:last-child {{ border-bottom: none; }}
  .grade-icon {{ font-size: 16px; flex-shrink: 0; margin-top: 1px; }}
  .grade-pass {{ color: var(--success); }}
  .grade-fail {{ color: var(--danger); }}
  .grade-text {{ flex: 1; }}
  .grade-evidence {{ color: var(--text-secondary); font-size: 12px; margin-top: 2px; }}

  .feedback-area {{
    width: 100%;
    min-height: 100px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
    color: var(--text);
    font-size: 14px;
    resize: vertical;
    font-family: inherit;
  }}
  .feedback-area:focus {{ outline: none; border-color: var(--accent); }}

  .submit-btn {{
    background: var(--accent);
    color: #fff;
    border: none;
    padding: 10px 20px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 600;
    margin-top: 12px;
  }}
  .submit-btn:hover {{ opacity: 0.9; }}

  .benchmark-table {{
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
    margin-bottom: 24px;
  }}
  .benchmark-table th, .benchmark-table td {{
    text-align: left;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
  }}
  .benchmark-table th {{
    color: var(--text-secondary);
    font-weight: 600;
    font-size: 12px;
    text-transform: uppercase;
  }}
  .benchmark-table tr:hover {{ background: rgba(48, 54, 61, 0.3); }}
  .delta-positive {{ color: var(--success); }}
  .delta-negative {{ color: var(--danger); }}

  .observations {{
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px;
  }}
  .observations li {{
    margin-left: 20px;
    margin-bottom: 6px;
    font-size: 13px;
  }}

  .collapsed {{ display: none; }}
  .toggle-btn {{
    background: none;
    border: none;
    color: var(--accent);
    cursor: pointer;
    font-size: 12px;
    padding: 0;
  }}

  @media (max-width: 900px) {{
    .comparison {{ grid-template-columns: 1fr; }}
  }}
</style>
</head>
<body>
<header>
  <h1>🔧 {html.escape(skill_name)}</h1>
  <span class="meta">{html.escape(iteration_dir)}</span>
</header>
<div class="tabs">
  <div class="tab active" onclick="showTab('outputs')">Outputs</div>
  <div class="tab" onclick="showTab('benchmark')">Benchmark</div>
</div>

<div class="content">
  <div id="outputs-panel" class="tab-panel active">
    <div class="eval-nav">
      <button id="prev-btn" onclick="prevEval()">← Previous</button>
      <span class="eval-counter" id="eval-counter">1 / 1</span>
      <button id="next-btn" onclick="nextEval()">Next →</button>
    </div>
    <div id="eval-content"></div>
  </div>

  <div id="benchmark-panel" class="tab-panel">
    <div id="benchmark-content"></div>
  </div>
</div>

<script>
const benchmark = {benchmark_json};
const evals = {evals_json};
let currentEval = 0;
let feedback = {{}};

function showTab(name) {{
  document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
  document.querySelectorAll('.tab-panel').forEach(p => p.classList.remove('active'));
  event.target.classList.add('active');
  document.getElementById(name + '-panel').classList.add('active');
}}

function escapeHtml(text) {{
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}}

function renderFiles(files) {{
  if (!files || Object.keys(files).length === 0) return '<p style="color: var(--text-secondary); font-size: 13px;">No output files</p>';
  return Object.entries(files).map(([name, content]) => `
    <div class="file-box">
      <div class="file-header">
        <span>${{escapeHtml(name)}}</span>
        <span style="color: var(--text-secondary);">${{content.length.toLocaleString()}} chars</span>
      </div>
      <div class="file-content">${{escapeHtml(content)}}</div>
    </div>
  `).join('');
}}

function renderGrading(grading) {{
  const expectations = grading.expectations || [];
  if (expectations.length === 0) return '<p style="color: var(--text-secondary); font-size: 13px;">No grading data</p>';
  return expectations.map(e => `
    <div class="grading-item">
      <span class="grade-icon ${{e.passed ? 'grade-pass' : 'grade-fail'}}">${{e.passed ? '✓' : '✗'}}</span>
      <div class="grade-text">
        <div>${{escapeHtml(e.text)}}</div>
        ${{e.evidence ? `<div class="grade-evidence">${{escapeHtml(e.evidence)}}</div>` : ''}}
      </div>
    </div>
  `).join('');
}}

function renderEval(index) {{
  const ev = evals[index];
  const runId = ev.name + '-with_skill';
  const currentFeedback = feedback[runId] || '';

  let html = `
    <div class="section">
      <div class="section-title">Prompt</div>
      <div class="prompt-box">${{escapeHtml(ev.prompt)}}</div>
    </div>

    <div class="section">
      <div class="section-title">Outputs</div>
      <div class="comparison">
        <div class="comparison-col">
          <h3>With Skill ${{ev.with_skill_timing.total_duration_seconds ? '(' + ev.with_skill_timing.total_duration_seconds + 's)' : ''}}</h3>
          ${{renderFiles(ev.with_skill_files)}}
        </div>
        <div class="comparison-col">
          <h3>${{ev.baseline_label.replace('_', ' ').replace(/\b\w/g, l => l.toUpperCase())}} ${{ev.baseline_timing.total_duration_seconds ? '(' + ev.baseline_timing.total_duration_seconds + 's)' : ''}}</h3>
          ${{renderFiles(ev.baseline_files)}}
        </div>
      </div>
    </div>
  `;

  if (ev.prev_with_skill_files && Object.keys(ev.prev_with_skill_files).length > 0) {{
    html += `
      <div class="section">
        <div class="section-title">Previous Output (Iteration ${{benchmark.iteration - 1}})</div>
        ${{renderFiles(ev.prev_with_skill_files)}}
      </div>
    `;
  }}

  if ((ev.with_skill_grading.expectations || []).length > 0) {{
    html += `
      <div class="section">
        <div class="section-title">
          Formal Grades
          <button class="toggle-btn" onclick="this.parentElement.nextElementSibling.classList.toggle('collapsed')">[toggle]</button>
        </div>
        <div class="collapsed">
          <div style="margin-bottom: 12px;"><strong>With Skill:</strong></div>
          ${{renderGrading(ev.with_skill_grading)}}
          <div style="margin: 12px 0;"><strong>Baseline:</strong></div>
          ${{renderGrading(ev.baseline_grading)}}
        </div>
      </div>
    `;
  }}

  html += `
    <div class="section">
      <div class="section-title">Feedback</div>
      <textarea class="feedback-area" id="feedback-${{index}}" placeholder="Enter your feedback here..." oninput="saveFeedback(${{index}})">${{escapeHtml(currentFeedback)}}</textarea>
    </div>
  `;

  document.getElementById('eval-content').innerHTML = html;
  document.getElementById('eval-counter').textContent = `${{index + 1}} / ${{evals.length}}`;
  document.getElementById('prev-btn').disabled = index === 0;
  document.getElementById('next-btn').disabled = index === evals.length - 1;
}}

function saveFeedback(index) {{
  const ev = evals[index];
  const runId = ev.name + '-with_skill';
  feedback[runId] = document.getElementById('feedback-' + index).value;
}}

function prevEval() {{ if (currentEval > 0) {{ currentEval--; renderEval(currentEval); }} }}
function nextEval() {{ if (currentEval < evals.length - 1) {{ currentEval++; renderEval(currentEval); }} }}

function renderBenchmark() {{
  if (!benchmark.configurations) {{
    document.getElementById('benchmark-content').innerHTML = '<p>No benchmark data</p>';
    return;
  }}

  let html = '<h2 style="margin-bottom: 16px;">Aggregate Metrics</h2>';

  html += '<table class="benchmark-table"><thead><tr><th>Configuration</th><th>Mean Pass Rate</th><th>Mean Tokens</th><th>Mean Duration</th></tr></thead><tbody>';
  benchmark.configurations.forEach(cfg => {{
    const agg = cfg.aggregate;
    html += `<tr>
      <td><strong>${{escapeHtml(cfg.name)}}</strong></td>
      <td>${{(agg.mean_pass_rate * 100).toFixed(1)}}% (±${{(agg.stddev_pass_rate * 100).toFixed(1)}}%)</td>
      <td>${{Math.round(agg.mean_tokens).toLocaleString()}} (±${{Math.round(agg.stddev_tokens).toLocaleString()}})</td>
      <td>${{(agg.mean_duration_ms / 1000).toFixed(1)}}s (±${{(agg.stddev_duration_ms / 1000).toFixed(1)}}s)</td>
    </tr>`;
  }});
  html += '</tbody></table>';

  if (benchmark.deltas) {{
    const d = benchmark.deltas;
    html += '<h2 style="margin-bottom: 16px;">Deltas (with_skill - baseline)</h2>';
    html += '<table class="benchmark-table"><thead><tr><th>Metric</th><th>Delta</th></tr></thead><tbody>';
    html += `<tr><td>Pass Rate</td><td class="${{d.pass_rate_delta >= 0 ? 'delta-positive' : 'delta-negative'}}">${{(d.pass_rate_delta * 100).toFixed(1)}}%</td></tr>`;
    html += `<tr><td>Tokens</td><td class="${{d.tokens_delta <= 0 ? 'delta-positive' : 'delta-negative'}}">${{Math.round(d.tokens_delta).toLocaleString()}}</td></tr>`;
    html += `<tr><td>Duration</td><td class="${{d.duration_delta <= 0 ? 'delta-positive' : 'delta-negative'}}">${{(d.duration_delta / 1000).toFixed(1)}}s</td></tr>`;
    html += '</tbody></table>';
  }}

  html += '<h2 style="margin-bottom: 16px;">Per-Eval Breakdown</h2>';
  benchmark.configurations.forEach(cfg => {{
    html += `<h3 style="margin: 16px 0 8px; font-size: 14px;">${{escapeHtml(cfg.name)}}</h3>`;
    html += '<table class="benchmark-table"><thead><tr><th>Eval</th><th>Pass Rate</th><th>Tokens</th><th>Duration</th></tr></thead><tbody>';
    cfg.evals.forEach(ev => {{
      html += `<tr>
        <td>${{escapeHtml(ev.eval_name)}}</td>
        <td>${{(ev.pass_rate * 100).toFixed(1)}}%</td>
        <td>${{ev.total_tokens.toLocaleString()}}</td>
        <td>${{(ev.duration_ms / 1000).toFixed(1)}}s</td>
      </tr>`;
    }});
    html += '</tbody></table>';
  }});

  if (benchmark.analyst_observations && benchmark.analyst_observations.length > 0) {{
    html += '<h2 style="margin-bottom: 16px;">Analyst Observations</h2>';
    html += '<ul class="observations">';
    benchmark.analyst_observations.forEach(obs => {{
      html += `<li>${{escapeHtml(obs)}}</li>`;
    }});
    html += '</ul>';
  }}

  html += `<div style="margin-top: 24px; padding-top: 16px; border-top: 1px solid var(--border);">
    <button class="submit-btn" onclick="downloadFeedback()">Submit All Reviews</button>
  </div>`;

  document.getElementById('benchmark-content').innerHTML = html;
}}

function downloadFeedback() {{
  const data = {{
    reviews: Object.entries(feedback).map(([run_id, text]) => ({{
      run_id,
      feedback: text,
      timestamp: new Date().toISOString()
    }})),
    status: "complete"
  }};
  const blob = new Blob([JSON.stringify(data, null, 2)], {{ type: 'application/json' }});
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = 'feedback.json';
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
  alert('Feedback downloaded as feedback.json');
}}

document.addEventListener('keydown', (e) => {{
  if (document.getElementById('outputs-panel').classList.contains('active')) {{
    if (e.key === 'ArrowLeft') prevEval();
    if (e.key === 'ArrowRight') nextEval();
  }}
}});

renderEval(0);
renderBenchmark();
</script>
</body>
</html>"""

    return html_template


def main():
    parser = argparse.ArgumentParser(description="Generate evaluation review viewer")
    parser.add_argument("workspace", help="Path to iteration-N directory")
    parser.add_argument("--skill-name", required=True, help="Skill name")
    parser.add_argument("--benchmark", help="Path to benchmark.json")
    parser.add_argument("--previous-workspace", help="Path to previous iteration for comparison")
    parser.add_argument("--static", help="Write standalone HTML to this path instead of starting server")
    parser.add_argument("--port", type=int, default=8080, help="Server port (default: 8080)")
    args = parser.parse_args()

    benchmark = load_json(args.benchmark) or {}

    html_content = generate_html(
        args.workspace,
        args.skill_name,
        benchmark,
        args.previous_workspace,
    )

    if args.static:
        output_path = os.path.abspath(args.static)
        os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)
        with open(output_path, "w", encoding="utf-8") as f:
            f.write(html_content)
        print(f"Static HTML written to: {output_path}")
        print("Open this file in a browser to review results.")
        print("Feedback will be downloaded as feedback.json when you click 'Submit All Reviews'.")
    else:
        import http.server
        import socketserver
        import tempfile
        import webbrowser

        # Write to temp file
        with tempfile.NamedTemporaryFile(mode="w", suffix=".html", delete=False, encoding="utf-8") as f:
            f.write(html_content)
            temp_path = f.name

        class Handler(http.server.SimpleHTTPRequestHandler):
            def do_GET(self):
                if self.path == "/" or self.path == "/index.html":
                    self.send_response(200)
                    self.send_header("Content-type", "text/html")
                    self.end_headers()
                    with open(temp_path, "rb") as f:
                        self.wfile.write(f.read())
                else:
                    super().do_GET()

            def log_message(self, format, *args):
                pass

        try:
            with socketserver.TCPServer(("", args.port), Handler) as httpd:
                print(f"Server running at http://localhost:{args.port}")
                print("Press Ctrl+C to stop")
                webbrowser.open(f"http://localhost:{args.port}")
                httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nServer stopped")
        finally:
            os.unlink(temp_path)


if __name__ == "__main__":
    main()
