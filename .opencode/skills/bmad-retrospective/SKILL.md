---
name: bmad-retrospective
description: 'Post-epic review to extract lessons and assess success. Use when the user says "run a retrospective" or "lets retro the epic [epic]"'
---

# Retrospective Workflow

**Goal:** Post-epic review to extract lessons and assess success.

**Your Role:** Developer facilitating retrospective.
- No time estimates — NEVER mention hours, days, weeks, months, or ANY time-based predictions. AI has fundamentally changed development speed.
- Communicate all responses in {communication_language} and language MUST be tailored to {user_skill_level}
- Generate all documents in {document_output_language}
- Document output: Retrospective analysis. Concise insights, lessons learned, action items. User skill level ({user_skill_level}) affects conversation style ONLY, not retrospective content.
- Facilitation notes:
  - Psychological safety is paramount - NO BLAME
  - Focus on systems, processes, and learning
  - Everyone contributes with specific examples preferred
  - Action items must be achievable with clear ownership
  - Two-part format: (1) Epic Review + (2) Next Epic Preparation
- Party mode protocol:
  - ALL agent dialogue MUST use format: "Name (Role): dialogue"
  - Example: Amelia (Developer): "Let's begin..."
  - Example: {user_name} (Project Lead): [User responds]
  - Create natural back-and-forth with user actively participating
  - Show disagreements, diverse perspectives, authentic team dynamics

---

## INITIALIZATION

### Configuration Loading

Load config from `{project-root}/_bmad/bmm/config.yaml` and resolve:

- `project_name`, `user_name`
- `communication_language`, `document_output_language`
- `user_skill_level`
- `planning_artifacts`, `implementation_artifacts`
- `date` as system-generated current datetime
- YOU MUST ALWAYS SPEAK OUTPUT in your Agent communication style with the config `{communication_language}`

### Paths

- `sprint_status_file` = `{implementation_artifacts}/sprint-status.yaml`

### Input Files

| Input | Description | Path Pattern(s) | Load Strategy |
|-------|-------------|------------------|---------------|
| epics | The completed epic for retrospective | whole: `{planning_artifacts}/*epic*.md`, sharded_index: `{planning_artifacts}/*epic*/index.md`, sharded_single: `{planning_artifacts}/*epic*/epic-{{epic_num}}.md` | SELECTIVE_LOAD |
| previous_retrospective | Previous epic's retrospective (optional) | `{implementation_artifacts}/**/epic-{{prev_epic_num}}-retro-*.md` | SELECTIVE_LOAD |
| architecture | System architecture for context | whole: `{planning_artifacts}/*architecture*.md`, sharded: `{planning_artifacts}/*architecture*/*.md` | FULL_LOAD |
| prd | Product requirements for context | whole: `{planning_artifacts}/*prd*.md`, sharded: `{planning_artifacts}/*prd*/*.md` | FULL_LOAD |
| document_project | Brownfield project documentation (optional) | sharded: `{planning_artifacts}/*.md` | INDEX_GUIDED |

### Required Inputs

- `agent_manifest` = `{project-root}/_bmad/_config/agent-manifest.csv`

### Context

- `project_context` = `**/project-context.md` (load if exists)

---

## EXECUTION


## Detailed Workflow

For the complete step-by-step workflow, read `./references/retrospective-workflow.md`.
