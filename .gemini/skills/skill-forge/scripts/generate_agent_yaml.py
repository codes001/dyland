#!/usr/bin/env python3
"""Generate or regenerate agents/agent.yaml for a skill."""

import argparse
import os
import sys

import yaml


def parse_interface_args(args_list: list[str]) -> dict[str, str]:
    """Parse --interface key=value arguments."""
    result = {}
    for arg in args_list or []:
        if '=' in arg:
            key, value = arg.split('=', 1)
            result[key.strip()] = value.strip()
    return result


def generate_yaml(display_name: str, short_description: str, default_prompt: str) -> str:
    """Generate agent.yaml content."""
    lines = [
        f"display_name: {display_name}",
        f"short_description: {short_description}",
        f"default_prompt: {default_prompt}",
    ]
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description="Generate agents/agent.yaml for a skill")
    parser.add_argument("skill_path", help="Path to the skill folder")
    parser.add_argument("--interface", action="append", required=True, help="Key=value pairs (display_name, short_description, default_prompt)")
    args = parser.parse_args()

    interface = parse_interface_args(args.interface)

    required = ["display_name", "short_description", "default_prompt"]
    missing = [k for k in required if k not in interface]
    if missing:
        print(f"ERROR: Missing required interface fields: {missing}")
        sys.exit(1)

    agents_dir = os.path.join(args.skill_path, "agents")
    os.makedirs(agents_dir, exist_ok=True)

    yaml_content = generate_yaml(
        interface["display_name"],
        interface["short_description"],
        interface["default_prompt"],
    )

    output_path = os.path.join(agents_dir, "agent.yaml")
    with open(output_path, "w") as f:
        f.write(yaml_content)

    print(f"Wrote {output_path}")


if __name__ == "__main__":
    main()
