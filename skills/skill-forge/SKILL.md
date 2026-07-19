---
name: skill-forge
description: >
  Create, edit, and optimize skills that extend AI agent capabilities with specialized
  knowledge, workflows, and bundled resources. Use this skill whenever the user wants to
  build a new skill, improve an existing skill, update skill descriptions, add scripts or
  references to a skill, evaluate skill performance, benchmark skills against baselines,
  optimize skill triggering accuracy, or package skills for distribution. Also use when
  the user mentions skill validation, test cases for skills, skill evals, skill iteration,
  or skill descriptions that need better triggering. This skill provides the complete
  pipeline from initial concept through rigorous evaluation to final packaging.
---

# Skill Forge

The complete pipeline for creating high-quality, well-architected, and rigorously-tested skills.

## What This Skill Provides

1. **Architectural discipline** — Progressive disclosure, context-efficient design, and resource taxonomy
2. **Rigorous evaluation** — Subagent testing, benchmark aggregation, and blind comparison
3. **Trigger optimization** — Data-driven description tuning for reliable skill invocation
4. **Production tooling** — Scripts for initialization, validation, benchmarking, and packaging

## Core Principles

### Concise Is Key

The context window is a public good. Skills share it with system prompts, conversation history, other skills, and the user request.

**Default assumption: the model is already very smart.** Only add context it doesn't already have. Challenge every paragraph: "Does this justify its token cost?"

Prefer concise examples over verbose explanations.

### Set Appropriate Degrees of Freedom

Match specificity to the task's fragility:

- **High freedom (text instructions)**: Multiple valid approaches, context-dependent decisions
- **Medium freedom (parameterized scripts)**: Preferred pattern exists, some variation acceptable
- **Low freedom (specific scripts, few parameters)**: Fragile operations, consistency critical

### Progressive Disclosure

Skills load in three levels to manage context efficiently:

1. **Metadata** (name + description) — Always in context (~100 words)
2. **SKILL.md body** — When skill triggers (<500 lines ideal)
3. **Bundled resources** — As needed (unlimited; scripts execute without loading)

**Key rule**: When approaching 500 lines in SKILL.md, add hierarchy and clear pointers to reference files. Keep only essential procedural instructions in SKILL.md; move detailed reference material, schemas, and examples to `references/`.

## Anatomy of a Skill

```
skill-name/
├── SKILL.md (required)
│   ├── YAML frontmatter (name, description required)
│   └── Markdown instructions
├── agents/
│   └── agent.yaml — UI metadata for skill lists
└── Bundled Resources (optional)
    ├── scripts/    — Executable code for deterministic tasks
    ├── references/ — Docs loaded into context as needed
    └── assets/     — Files used in output (templates, icons, fonts)
```

**SKILL.md frontmatter** (`name` + `description`) is the only triggering mechanism. Include ALL "when to use" information here — not in the body. The body is only loaded after triggering.

**What NOT to include**: README.md, INSTALLATION_GUIDE.md, CHANGELOG.md, or any auxiliary documentation. The skill should only contain information needed for an AI agent to do the job.

## Skill Creation Process

Follow these steps in order, skipping only when clearly inapplicable:

1. Understand the skill with concrete examples
2. Plan reusable skill contents (scripts, references, assets)
3. Initialize the skill (`scripts/init_skill.py`)
4. Edit the skill (implement resources and write SKILL.md)
5. Validate the skill (`scripts/quick_validate.py`)
6. Iterate based on real usage (test → evaluate → improve → repeat)

### Step 1: Understand with Concrete Examples

Start by understanding the user's intent. Extract from conversation history: tools used, sequence of steps, corrections made, input/output formats.

Key questions:
1. What should this skill enable the agent to do?
2. When should this skill trigger? (user phrases/contexts)
3. What's the expected output format?
4. Should we set up test cases? (Objective outputs → yes; subjective → optional)

Ask about edge cases, input/output formats, example files, success criteria, and dependencies. Conclude when you have a clear sense of supported functionality.

### Step 2: Plan Reusable Contents

For each concrete example, analyze:
1. How would you execute this from scratch?
2. What scripts, references, or assets would help when repeating this?

**Examples:**
- PDF rotation → `scripts/rotate_pdf.py`
- Frontend boilerplate → `assets/hello-world/` template
- Database schemas → `references/schema.md`

List all reusable resources before writing any code.

### Step 3: Initialize the Skill

Run the initialization script:

```bash
python -m scripts.init_skill <skill-name> --path <output-directory> [--resources scripts,references,assets] [--examples]
```

This creates:
- Skill directory with proper naming (lowercase, hyphens, <64 chars)
- SKILL.md template with frontmatter and TODO placeholders
- `agents/agent.yaml` with display metadata
- Optional resource directories

Generate `display_name`, `short_description`, and `default_prompt` by reading the skill concept, then pass them as `--interface key=value`.

### Step 4: Edit the Skill

#### Start with Resources

Implement `scripts/`, `references/`, and `assets/` first. Test scripts by actually running them. Delete any placeholder files that aren't needed.

#### Write SKILL.md

**Frontmatter:**
- `name`: Skill identifier (lowercase, hyphens, <64 chars)
- `description`: Primary triggering mechanism. Be comprehensive and slightly "pushy" to combat undertriggering. Include what the skill does AND specific contexts for when to use it.

**Body:**
- Use imperative/infinitive form
- Define output formats explicitly when strict templates are needed
- Include examples formatted as: **Example N:** Input / Output
- Explain the *why* behind instructions, not just the *what*
- Avoid ALL CAPS directives unless truly critical

**Reference files:**
- For large reference files (>100 lines), include a table of contents
- Reference clearly from SKILL.md with guidance on when to read them
- Avoid duplication between SKILL.md and references

### Step 5: Validate

Run validation to catch basic issues early:

```bash
python -m scripts.quick_validate <path/to/skill-folder>
```

Checks YAML frontmatter format, required fields, and naming rules. Fix issues and re-run.

### Step 6: Iterate with Evaluation

This is the core improvement loop. Do NOT skip evaluation — a well-structured but untested skill is likely ineffective.

#### 6a. Create Test Cases

Draft 2-3 realistic test prompts. Save to `evals/evals.json`:

```json
{
  "skill_name": "example-skill",
  "evals": [
    {
      "id": 1,
      "prompt": "User's task prompt",
      "expected_output": "Description of expected result",
      "files": []
    }
  ]
}
```

Share with user for confirmation before proceeding.

#### 6b. Run Tests (With-Skill + Baseline)

For each test case, spawn TWO subagents in the SAME turn:

**With-skill run:**
- Skill path: `<path-to-skill>`
- Task: `<eval prompt>`
- Save outputs to: `<workspace>/iteration-N/eval-<name>/with_skill/outputs/`

**Baseline run:**
- New skill: no skill at all (save to `without_skill/outputs/`)
- Existing skill: old version snapshot (save to `old_skill/outputs/`)

Write `eval_metadata.json` for each test case. Capture timing data from subagent notifications into `timing.json`.

**See `references/eval_workflow.md` for complete details.**

#### 6c. Draft Assertions

While runs execute, draft quantitative assertions. Good assertions are objectively verifiable with descriptive names. Subjective skills (writing style, art) are better evaluated qualitatively.

Update `eval_metadata.json` and `evals/evals.json` with assertions.

#### 6d. Grade, Aggregate, and Review

Once runs complete:

1. **Grade each run** — Evaluate assertions against outputs. Save to `grading.json`:
   ```json
   {"expectations": [{"text": "...", "passed": true, "evidence": "..."}]}
   ```
   Use scripts for programmatic checks rather than eyeballing.

2. **Aggregate** — Run:
   ```bash
   python -m scripts.aggregate_benchmark <workspace>/iteration-N --skill-name <name>
   ```
   Produces `benchmark.json` and `benchmark.md` with pass rates, timing, and tokens.

3. **Analyze** — Read benchmark data for patterns: non-discriminating assertions, high-variance evals, time/token tradeoffs.

4. **Launch viewer** — Generate the eval viewer for human review:
   ```bash
   python -m eval-viewer.generate_review.py <workspace>/iteration-N --skill-name "my-skill" --benchmark <workspace>/iteration-N/benchmark.json
   ```
   For iteration 2+, add `--previous-workspace <workspace>/iteration-<N-1>`.

   In headless environments, use `--static <output_path>` to write a standalone HTML file.

5. **Tell the user**: "I've opened the results. Review the Outputs and Benchmark tabs, then let me know your feedback."

#### 6e. Read Feedback and Improve

When the user is done, read `feedback.json`:

```json
{"reviews": [{"run_id": "...", "feedback": "...", "timestamp": "..."}]}
```

Empty feedback means the user thought it was fine. Focus improvements on specific complaints.

**How to improve:**
1. **Generalize** — Don't overfit to test cases; ensure changes help across all usage
2. **Keep lean** — Remove instructions that aren't pulling their weight
3. **Explain why** — Use theory of mind; explain reasoning instead of rigid MUSTs
4. **Bundle repeated work** — If subagents repeatedly write similar helpers, add them to `scripts/`

#### 6f. Repeat

Apply improvements, rerun all test cases into `iteration-<N+1>/`, launch viewer with `--previous-workspace`, and repeat until:
- The user says they're happy
- Feedback is all empty
- You're not making meaningful progress

## Description Optimization

After the skill works well, optimize the description for triggering accuracy.

### 1. Generate Trigger Eval Queries

Create 20 realistic queries (mix of should-trigger and should-not-trigger). Save as JSON:

```json
[
  {"query": "...", "should_trigger": true},
  {"query": "...", "should_trigger": false}
]
```

Focus on edge cases and near-misses. See `references/eval_workflow.md` for detailed guidance.

### 2. Review with User

Present eval set using `assets/eval_review.html`. User can edit, toggle, add/remove entries.

### 3. Run Optimization Loop

```bash
python -m scripts.run_loop   --eval-set <path-to-trigger-eval.json>   --skill-path <path-to-skill>   --model <model-id>   --max-iterations 5   --verbose
```

This splits into train/test, evaluates current description (3 runs per query), calls the model to propose improvements, and iterates. Returns `best_description` selected by test score.

### 4. Apply Result

Update SKILL.md frontmatter with `best_description`. Show before/after and report scores.

## Packaging

When complete, package the skill:

```bash
python -m scripts.package_skill <path/to/skill-folder>
```

Direct the user to the resulting `.skill` file for installation.

## Reference Index

- **`references/eval_workflow.md`** — Complete evaluation workflow: subagent spawning, grading, benchmark aggregation, viewer generation, and blind comparison
- **`references/schemas.md`** — JSON schemas for `evals.json`, `grading.json`, `benchmark.json`, `feedback.json`
- **`references/agents.md`** — Instructions for grader, analyzer, and comparator subagents
- **`references/agent_yaml.md`** — Field definitions and examples for `agents/agent.yaml`
