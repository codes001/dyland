#!/usr/bin/env python3
"""Validate a skill folder for basic structural and metadata issues."""

import argparse
import os
import re
import sys

import yaml


def validate_skill_name(name: str) -> tuple[bool, str]:
    """Check skill name conventions."""
    if len(name) > 64:
        return False, f"Name '{name}' exceeds 64 characters ({len(name)} chars)"
    if not re.match(r'^[a-z0-9-]+$', name):
        return False, f"Name '{name}' must be lowercase letters, digits, and hyphens only"
    return True, "OK"


def validate_frontmatter(skill_path: str) -> list[str]:
    """Validate YAML frontmatter in SKILL.md."""
    errors = []
    skill_md_path = os.path.join(skill_path, "SKILL.md")

    if not os.path.exists(skill_md_path):
        errors.append("SKILL.md is missing")
        return errors

    with open(skill_md_path, "r") as f:
        content = f.read()

    # Check for YAML frontmatter
    if not content.startswith("---"):
        errors.append("SKILL.md must start with YAML frontmatter (---)")
        return errors

    parts = content.split("---", 2)
    if len(parts) < 3:
        errors.append("SKILL.md frontmatter not properly closed (missing second ---)")
        return errors

    frontmatter_text = parts[1].strip()
    body = parts[2].strip()

    try:
        frontmatter = yaml.safe_load(frontmatter_text) or {}
    except yaml.YAMLError as e:
        errors.append(f"Invalid YAML frontmatter: {e}")
        return errors

    # Required fields
    if "name" not in frontmatter:
        errors.append("Frontmatter missing required field: 'name'")
    else:
        valid, msg = validate_skill_name(frontmatter["name"])
        if not valid:
            errors.append(msg)

    if "description" not in frontmatter:
        errors.append("Frontmatter missing required field: 'description'")
    elif not frontmatter["description"] or not str(frontmatter["description"]).strip():
        errors.append("Frontmatter 'description' is empty")

    # Forbidden fields
    allowed_fields = {"name", "description"}
    extra_fields = set(frontmatter.keys()) - allowed_fields
    if extra_fields:
        errors.append(f"Frontmatter contains extra fields (only name and description allowed): {extra_fields}")

    # Body checks
    if not body:
        errors.append("SKILL.md body is empty")

    line_count = len(body.splitlines())
    if line_count > 500:
        errors.append(f"SKILL.md body is {line_count} lines (recommended: <500). Consider splitting into reference files.")

    return errors


def validate_structure(skill_path: str) -> list[str]:
    """Validate directory structure."""
    errors = []

    # Check for forbidden files
    forbidden = ["README.md", "INSTALLATION_GUIDE.md", "QUICK_REFERENCE.md", "CHANGELOG.md"]
    for fname in forbidden:
        if os.path.exists(os.path.join(skill_path, fname)):
            errors.append(f"Forbidden file detected: {fname} (skills should not contain auxiliary documentation)")

    # Check directory name matches skill name
    dir_name = os.path.basename(os.path.normpath(skill_path))
    skill_md_path = os.path.join(skill_path, "SKILL.md")
    if os.path.exists(skill_md_path):
        with open(skill_md_path, "r") as f:
            content = f.read()
        parts = content.split("---", 2)
        if len(parts) >= 3:
            try:
                fm = yaml.safe_load(parts[1]) or {}
                if "name" in fm and fm["name"] != dir_name:
                    errors.append(f"Directory name '{dir_name}' does not match skill name '{fm['name']}'")
            except Exception:
                pass

    return errors


def main():
    parser = argparse.ArgumentParser(description="Validate a skill folder")
    parser.add_argument("skill_path", help="Path to the skill folder")
    args = parser.parse_args()

    if not os.path.isdir(args.skill_path):
        print(f"ERROR: {args.skill_path} is not a directory")
        sys.exit(1)

    all_errors = []
    all_errors.extend(validate_frontmatter(args.skill_path))
    all_errors.extend(validate_structure(args.skill_path))

    if all_errors:
        print(f"Validation FAILED ({len(all_errors)} issue(s)):")
        for err in all_errors:
            print(f"  - {err}")
        sys.exit(1)
    else:
        print("Validation PASSED")
        sys.exit(0)


if __name__ == "__main__":
    main()
