#!/usr/bin/env python3
"""Refactor bmad-retrospective to reduce SKILL.md size."""

import shutil
from pathlib import Path

skill_dir = Path("/Users/jerryinyang/Code/logiq/.opencode/skills/bmad-retrospective")
workflow_md = skill_dir / "workflow.md"
skill_md = skill_dir / "SKILL.md"

# Backup
shutil.copy(skill_md, skill_dir / "SKILL.md.backup2")

# Read workflow.md
with open(workflow_md, 'r') as f:
    lines = f.readlines()

# Find line where <workflow> starts
workflow_start = None
for i, line in enumerate(lines):
    if line.strip() == '<workflow>':
        workflow_start = i
        break

if workflow_start is None:
    print("Could not find <workflow> tag")
    exit(1)

# Split: keep lines before <workflow> as essential
essential_lines = lines[:workflow_start]  # includes the <workflow> tag? we'll exclude
# Actually we want to keep up to line before <workflow>
essential_lines = lines[:workflow_start]
# Add a note referencing the detailed workflow
essential_lines.append('\n')
essential_lines.append('## Detailed Workflow\n')
essential_lines.append('\n')
essential_lines.append('For the complete step-by-step workflow, read `./references/retrospective-workflow.md`.\n')

# Write new SKILL.md (preserve frontmatter)
with open(skill_md, 'r') as f:
    skill_content = f.read()

# Extract frontmatter (first 5 lines)
skill_lines = skill_content.split('\n')
frontmatter_lines = []
in_frontmatter = False
for line in skill_lines:
    if line.strip() == '---':
        in_frontmatter = not in_frontmatter
        frontmatter_lines.append(line)
        if not in_frontmatter:
            break
    if in_frontmatter:
        frontmatter_lines.append(line)

frontmatter = '\n'.join(frontmatter_lines)

# Write new SKILL.md
with open(skill_md, 'w') as f:
    f.write(frontmatter + '\n\n')
    f.writelines(essential_lines)

# Write detailed workflow to references
references_dir = skill_dir / "references"
references_dir.mkdir(exist_ok=True)
with open(references_dir / "retrospective-workflow.md", 'w') as f:
    f.writelines(lines[workflow_start:])

print("Refactored bmad-retrospective")