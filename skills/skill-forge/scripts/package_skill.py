#!/usr/bin/env python3
"""Package a skill folder into a .skill file for distribution."""

import argparse
import os
import sys
import zipfile


def package_skill(skill_path: str, output_dir: str | None = None) -> str:
    """Package skill folder into a .skill file."""
    skill_path = os.path.abspath(skill_path)
    skill_name = os.path.basename(os.path.normpath(skill_path))

    if not os.path.isdir(skill_path):
        raise ValueError(f"Not a directory: {skill_path}")

    if not os.path.exists(os.path.join(skill_path, "SKILL.md")):
        raise ValueError(f"SKILL.md not found in {skill_path}")

    out_dir = os.path.abspath(output_dir) if output_dir else os.path.dirname(skill_path)
    os.makedirs(out_dir, exist_ok=True)

    output_path = os.path.join(out_dir, f"{skill_name}.skill")

    with zipfile.ZipFile(output_path, "w", zipfile.ZIP_DEFLATED) as zf:
        for root, dirs, files in os.walk(skill_path):
            # Skip hidden directories and __pycache__
            dirs[:] = [d for d in dirs if not d.startswith(".") and d != "__pycache__"]

            for file in files:
                if file.startswith("."):
                    continue
                file_path = os.path.join(root, file)
                arcname = os.path.relpath(file_path, skill_path)
                zf.write(file_path, arcname)

    return output_path


def main():
    parser = argparse.ArgumentParser(description="Package a skill into a .skill file")
    parser.add_argument("skill_path", help="Path to the skill folder")
    parser.add_argument("--output", "-o", help="Output directory (default: same as skill parent)")
    parser.add_argument("--no-validate", action="store_true", help="Skip validation before packaging")
    args = parser.parse_args()

    if not args.no_validate:
        try:
            from scripts import quick_validate
            errors = quick_validate.validate_frontmatter(args.skill_path)
            errors += quick_validate.validate_structure(args.skill_path)
            if errors:
                print("Validation failed:")
                for err in errors:
                    print(f"  - {err}")
                print("\nUse --no-validate to package anyway.")
                sys.exit(1)
            print("Validation passed.")
        except ImportError:
            print("Warning: Could not import quick_validate, skipping validation.")

    try:
        output_path = package_skill(args.skill_path, args.output)
        print(f"Packaged skill to: {output_path}")
    except ValueError as e:
        print(f"ERROR: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
