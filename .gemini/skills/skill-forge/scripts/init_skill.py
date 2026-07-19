#!/usr/bin/env python3
"""Initialize a new skill directory with proper structure and templates."""

import argparse
import os
import re
import sys


def validate_skill_name(name: str) -> bool:
    """Check if skill name follows conventions."""
    if len(name) > 64:
        return False
    return bool(re.match(r'^[a-z0-9-]+$', name))


def generate_skill_md(name: str, description: str = "") -> str:
    """Generate SKILL.md template."""
    desc = description or f"Description for {name}. Update this with what the skill does and when to use it."
    return f"""---
name: {name}
description: >
  {desc}
---

# {name}

## Overview

Brief description of what this skill does.

## When to Use This Skill

- Scenario 1
- Scenario 2
- Scenario 3

## Quick Start

Basic usage example or first step.

## Detailed Instructions

### Step 1: ...

Instructions here.

### Step 2: ...

Instructions here.

## Examples

**Example 1:**
Input: ...
Output: ...

## References

- `references/example.md` — Description of when to read this
"""


def generate_agent_yaml(display_name: str = "", short_description: str = "", default_prompt: str = "") -> str:
    """Generate agents/agent.yaml template."""
    dn = display_name or "Skill Display Name"
    sd = short_description or "Short description for UI lists"
    dp = default_prompt or "Default prompt suggestion"
    return f"""display_name: {dn}
short_description: {sd}
default_prompt: {dp}
"""


def parse_interface_args(args_list: list[str]) -> dict[str, str]:
    """Parse --interface key=value arguments."""
    result = {}
    for arg in args_list or []:
        if '=' in arg:
            key, value = arg.split('=', 1)
            result[key.strip()] = value.strip()
    return result


def main():
    parser = argparse.ArgumentParser(description="Initialize a new skill")
    parser.add_argument("skill_name", help="Skill identifier (lowercase, hyphens, <64 chars)")
    parser.add_argument("--path", required=True, help="Output directory for the skill")
    parser.add_argument("--resources", help="Comma-separated list: scripts,references,assets")
    parser.add_argument("--examples", action="store_true", help="Add example placeholder files")
    parser.add_argument("--interface", action="append", help="Key=value pairs for agent.yaml (e.g., display_name=My Skill)")

    args = parser.parse_args()

    if not validate_skill_name(args.skill_name):
        print(f"ERROR: Skill name '{args.skill_name}' must be lowercase, hyphenated, and under 64 characters.")
        sys.exit(1)

    skill_dir = os.path.join(args.path, args.skill_name)

    if os.path.exists(skill_dir):
        print(f"ERROR: Directory {skill_dir} already exists.")
        sys.exit(1)

    # Create directories
    os.makedirs(skill_dir, exist_ok=True)
    os.makedirs(os.path.join(skill_dir, "agents"), exist_ok=True)

    interface = parse_interface_args(args.interface)

    # Write SKILL.md
    skill_md = generate_skill_md(
        args.skill_name,
        interface.get("description", "")
    )
    with open(os.path.join(skill_dir, "SKILL.md"), "w") as f:
        f.write(skill_md)

    # Write agents/agent.yaml
    agent_yaml = generate_agent_yaml(
        interface.get("display_name", ""),
        interface.get("short_description", ""),
        interface.get("default_prompt", "")
    )
    with open(os.path.join(skill_dir, "agents", "agent.yaml"), "w") as f:
        f.write(agent_yaml)

    # Create resource directories
    if args.resources:
        for resource in args.resources.split(","):
            resource = resource.strip()
            if resource in ("scripts", "references", "assets"):
                os.makedirs(os.path.join(skill_dir, resource), exist_ok=True)
                print(f"Created {resource}/")

    # Add example files if requested
    if args.examples:
        examples_dir = os.path.join(skill_dir, "references")
        os.makedirs(examples_dir, exist_ok=True)
        with open(os.path.join(examples_dir, "example.md"), "w") as f:
            f.write("# Example Reference\n\nReplace this with actual reference material.\n")
        print("Created references/example.md (placeholder)")

    print(f"\nSkill '{args.skill_name}' initialized at {skill_dir}")
    print("Next steps:")
    print("  1. Edit SKILL.md with your skill's instructions")
    print("  2. Add scripts, references, and assets as needed")
    print("  3. Run: python -m scripts.quick_validate " + skill_dir)


if __name__ == "__main__":
    main()
