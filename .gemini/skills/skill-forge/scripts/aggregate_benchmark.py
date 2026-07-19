#!/usr/bin/env python3
"""Aggregate evaluation results into benchmark.json and benchmark.md."""

import argparse
import json
import math
import os
import sys
from datetime import datetime, timezone


def load_json(path: str) -> dict | None:
    """Load JSON file if it exists."""
    if not os.path.exists(path):
        return None
    try:
        with open(path, "r") as f:
            return json.load(f)
    except (json.JSONDecodeError, IOError):
        return None


def calc_mean(values: list[float]) -> float:
    """Calculate mean."""
    return sum(values) / len(values) if values else 0.0


def calc_stddev(values: list[float]) -> float:
    """Calculate population standard deviation."""
    if len(values) < 2:
        return 0.0
    mean = calc_mean(values)
    variance = sum((x - mean) ** 2 for x in values) / len(values)
    return math.sqrt(variance)


def find_evals(iteration_dir: str) -> list[dict]:
    """Find all eval directories in an iteration."""
    evals = []
    if not os.path.isdir(iteration_dir):
        return evals

    for item in sorted(os.listdir(iteration_dir)):
        item_path = os.path.join(iteration_dir, item)
        if not os.path.isdir(item_path) or not item.startswith("eval-"):
            continue

        eval_data = {"dir_name": item, "configs": {}}

        for config in ["with_skill", "without_skill", "old_skill"]:
            config_dir = os.path.join(item_path, config)
            if not os.path.isdir(config_dir):
                continue

            timing = load_json(os.path.join(config_dir, "timing.json")) or {}
            grading = load_json(os.path.join(config_dir, "grading.json")) or {}
            metadata = load_json(os.path.join(item_path, "eval_metadata.json")) or {}

            expectations = grading.get("expectations", [])
            passed = sum(1 for e in expectations if e.get("passed", False))
            total = len(expectations)
            pass_rate = passed / total if total > 0 else 0.0

            eval_data["configs"][config] = {
                "timing": timing,
                "grading": grading,
                "pass_rate": pass_rate,
                "total_assertions": total,
                "passed_assertions": passed,
                "eval_name": metadata.get("eval_name", item),
                "eval_id": metadata.get("eval_id", 0),
            }

        evals.append(eval_data)

    return evals


def aggregate_config(evals: list[dict], config_name: str) -> dict:
    """Aggregate metrics for a configuration across all evals."""
    config_evals = []
    pass_rates = []
    tokens = []
    durations = []

    for eval_data in evals:
        if config_name not in eval_data["configs"]:
            continue
        cfg = eval_data["configs"][config_name]

        entry = {
            "eval_id": cfg["eval_id"],
            "eval_name": cfg["eval_name"],
            "pass_rate": cfg["pass_rate"],
            "total_tokens": cfg["timing"].get("total_tokens", 0),
            "duration_ms": cfg["timing"].get("duration_ms", 0),
            "total_assertions": cfg["total_assertions"],
            "passed_assertions": cfg["passed_assertions"],
        }
        config_evals.append(entry)
        pass_rates.append(cfg["pass_rate"])
        tokens.append(cfg["timing"].get("total_tokens", 0))
        durations.append(cfg["timing"].get("duration_ms", 0))

    return {
        "name": config_name,
        "evals": config_evals,
        "aggregate": {
            "mean_pass_rate": calc_mean(pass_rates),
            "stddev_pass_rate": calc_stddev(pass_rates),
            "mean_tokens": calc_mean(tokens),
            "stddev_tokens": calc_stddev(tokens),
            "mean_duration_ms": calc_mean(durations),
            "stddev_duration_ms": calc_stddev(durations),
        }
    }


def calc_deltas(with_skill: dict, baseline: dict) -> dict:
    """Calculate deltas (with_skill - baseline)."""
    w = with_skill["aggregate"]
    b = baseline["aggregate"]
    return {
        "pass_rate_delta": w["mean_pass_rate"] - b["mean_pass_rate"],
        "tokens_delta": w["mean_tokens"] - b["mean_tokens"],
        "duration_delta": w["mean_duration_ms"] - b["mean_duration_ms"],
    }


def generate_markdown(benchmark: dict) -> str:
    """Generate human-readable benchmark report."""
    lines = []
    lines.append(f"# Benchmark Report: {benchmark['skill_name']}")
    lines.append(f"**Iteration:** {benchmark['iteration']}  ")
    lines.append(f"**Timestamp:** {benchmark['timestamp']}\n")

    for config in benchmark["configurations"]:
        lines.append(f"## {config['name']}")
        agg = config["aggregate"]
        lines.append(f"- **Mean Pass Rate:** {agg['mean_pass_rate']:.2%} (±{agg['stddev_pass_rate']:.2%})")
        lines.append(f"- **Mean Tokens:** {agg['mean_tokens']:,.0f} (±{agg['stddev_tokens']:,.0f})")
        lines.append(f"- **Mean Duration:** {agg['mean_duration_ms']/1000:.1f}s (±{agg['stddev_duration_ms']/1000:.1f}s)")
        lines.append("")
        lines.append("| Eval | Pass Rate | Tokens | Duration |")
        lines.append("|------|-----------|--------|----------|")
        for ev in config["evals"]:
            lines.append(f"| {ev['eval_name']} | {ev['pass_rate']:.2%} | {ev['total_tokens']:,} | {ev['duration_ms']/1000:.1f}s |")
        lines.append("")

    deltas = benchmark.get("deltas", {})
    lines.append("## Deltas (with_skill - baseline)")
    lines.append(f"- **Pass Rate:** {deltas.get('pass_rate_delta', 0):+.2%}")
    lines.append(f"- **Tokens:** {deltas.get('tokens_delta', 0):+,.0f}")
    lines.append(f"- **Duration:** {deltas.get('duration_delta', 0)/1000:+.1f}s")
    lines.append("")

    observations = benchmark.get("analyst_observations", [])
    if observations:
        lines.append("## Analyst Observations")
        for obs in observations:
            lines.append(f"- {obs}")
        lines.append("")

    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description="Aggregate benchmark results")
    parser.add_argument("iteration_dir", help="Path to iteration-N directory")
    parser.add_argument("--skill-name", required=True, help="Skill name for the report")
    args = parser.parse_args()

    if not os.path.isdir(args.iteration_dir):
        print(f"ERROR: {args.iteration_dir} is not a directory")
        sys.exit(1)

    # Extract iteration number from directory name
    dir_name = os.path.basename(os.path.normpath(args.iteration_dir))
    iteration = 1
    if dir_name.startswith("iteration-"):
        try:
            iteration = int(dir_name.split("-")[1])
        except ValueError:
            pass

    evals = find_evals(args.iteration_dir)
    if not evals:
        print("WARNING: No eval directories found")

    # Determine which configurations exist
    all_configs = set()
    for e in evals:
        all_configs.update(e["configs"].keys())

    # Order: with_skill first, then baseline
    config_order = []
    if "with_skill" in all_configs:
        config_order.append("with_skill")
    for cfg in sorted(all_configs):
        if cfg != "with_skill":
            config_order.append(cfg)

    configurations = [aggregate_config(evals, cfg) for cfg in config_order]

    # Calculate deltas if both with_skill and baseline exist
    deltas = {}
    if len(configurations) >= 2:
        deltas = calc_deltas(configurations[0], configurations[1])

    benchmark = {
        "skill_name": args.skill_name,
        "iteration": iteration,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "configurations": configurations,
        "deltas": deltas,
        "analyst_observations": [],
    }

    # Write benchmark.json
    json_path = os.path.join(args.iteration_dir, "benchmark.json")
    with open(json_path, "w") as f:
        json.dump(benchmark, f, indent=2)
    print(f"Wrote {json_path}")

    # Write benchmark.md
    md_path = os.path.join(args.iteration_dir, "benchmark.md")
    with open(md_path, "w") as f:
        f.write(generate_markdown(benchmark))
    print(f"Wrote {md_path}")

    print("\nDone. Add analyst observations to benchmark.json['analyst_observations'] before viewing.")


if __name__ == "__main__":
    main()
