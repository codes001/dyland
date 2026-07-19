# Evaluation Workflow

Complete guide for running skill evaluations using subagent testing.

## Directory Structure

```
<skill-name>-workspace/
├── iteration-1/
│   ├── eval-<name>/
│   │   ├── with_skill/outputs/
│   │   ├── without_skill/outputs/  (or old_skill/outputs/)
│   │   ├── eval_metadata.json
│   │   ├── timing.json
│   │   └── grading.json
│   ├── benchmark.json
│   └── benchmark.md
├── iteration-2/
│   └── ...
└── feedback.json
```

## Step 1: Spawn All Runs (Same Turn)

Launch with-skill AND baseline subagents simultaneously. Do not spawn sequentially.

**With-skill run instructions:**
- Skill path: `<path-to-skill>`
- Task: `<eval prompt>`
- Input files: `<eval files or "none">`
- Save outputs to: `<workspace>/iteration-N/eval-<name>/with_skill/outputs/`
- Outputs to save: `<what the user cares about>`

**Baseline run:**
- New skill: same prompt, no skill path, save to `without_skill/outputs/`
- Existing skill: snapshot old version to `<workspace>/skill-snapshot/`, point baseline there, save to `old_skill/outputs/`

Create `eval_metadata.json`:

```json
{
  "eval_id": 0,
  "eval_name": "descriptive-name-here",
  "prompt": "The user's task prompt",
  "assertions": []
}
```

Use descriptive names for eval directories, not just "eval-0".

## Step 2: Draft Assertions (While Runs Execute)

Don't wait — draft quantitative assertions during run execution.

Good assertions are:
- Objectively verifiable
- Have descriptive names that read clearly in the benchmark viewer
- Focused on outcomes, not implementation details

Subjective skills (writing style, design quality) are better evaluated qualitatively.

Update `eval_metadata.json` and `evals/evals.json` with assertions once drafted.

## Step 3: Capture Timing Data

When subagent tasks complete, extract `total_tokens` and `duration_ms` from notifications. Save immediately to `timing.json`:

```json
{
  "total_tokens": 84852,
  "duration_ms": 23332,
  "total_duration_seconds": 23.3
}
```

This data is only available in task notifications and is not persisted elsewhere.

## Step 4: Grade Each Run

Spawn a grader subagent (or grade inline) that reads `agents/grader.md` and evaluates each assertion against outputs. Save results to `grading.json`:

```json
{
  "expectations": [
    {
      "text": "Output contains a chart with axis labels",
      "passed": true,
      "evidence": "The PNG output shows x-axis labeled 'Date' and y-axis labeled 'Revenue'"
    }
  ]
}
```

**Critical**: The `grading.json` expectations array MUST use fields `text`, `passed`, and `evidence`. The viewer depends on these exact field names.

For programmatic assertions, write and run a script rather than eyeballing.

## Step 5: Aggregate Benchmark

Run the aggregation script:

```bash
python -m scripts.aggregate_benchmark <workspace>/iteration-N --skill-name <name>
```

This produces:
- `benchmark.json` — Machine-readable results
- `benchmark.md` — Human-readable summary

Both include pass_rate, time, and tokens for each configuration, with mean ± stddev and deltas. Place each with_skill version before its baseline counterpart.

## Step 6: Analyst Pass

Read benchmark data and surface hidden patterns:

- **Non-discriminating assertions** — Always pass regardless of skill (remove or refine)
- **High-variance evals** — Possibly flaky; investigate root cause
- **Time/token tradeoffs** — Is the skill saving time but costing tokens, or vice versa?
- **Baseline surprises** — Sometimes baseline outperforms; understand why

See `references/agents.md` (analyzer section) for detailed guidance.

## Step 7: Launch Eval Viewer

Generate the review interface:

```bash
python -m eval-viewer.generate_review.py \
  <workspace>/iteration-N \
  --skill-name "my-skill" \
  --benchmark <workspace>/iteration-N/benchmark.json
```

For iteration 2+:
```bash
python -m eval-viewer.generate_review.py \
  <workspace>/iteration-N \
  --skill-name "my-skill" \
  --benchmark <workspace>/iteration-N/benchmark.json \
  --previous-workspace <workspace>/iteration-<N-1>
```

**Headless environments** (no display or browser):
```bash
python -m eval-viewer.generate_review.py \
  <workspace>/iteration-N \
  --skill-name "my-skill" \
  --benchmark <workspace>/iteration-N/benchmark.json \
  --static <output_path>/review.html
```

The viewer creates a standalone HTML file. Feedback downloads as `feedback.json` when the user clicks "Submit All Reviews". Copy `feedback.json` into the workspace directory for the next iteration.

## What the User Sees

**Outputs tab:**
- Prompt, output files (rendered inline), previous output (iteration 2+)
- Formal grades (collapsed), feedback textbox, previous feedback

**Benchmark tab:**
- Stats summary: pass rates, timing, token usage
- Per-eval breakdowns and analyst observations

Navigation via prev/next buttons or arrow keys.

## Step 8: Read Feedback

When the user is done, read `feedback.json`:

```json
{
  "reviews": [
    {"run_id": "eval-0-with_skill", "feedback": "the chart is missing axis labels", "timestamp": "..."},
    {"run_id": "eval-1-with_skill", "feedback": "", "timestamp": "..."}
  ],
  "status": "complete"
}
```

Empty feedback means the user thought it was fine. Focus improvements on specific complaints.

Kill the viewer server when done:
```bash
kill $VIEWER_PID 2>/dev/null
```

## Blind Comparison (Advanced)

For rigorous comparison between two skill versions:

1. Give two outputs to an independent agent without revealing which is which
2. Let it judge quality based on predefined criteria
3. Analyze why the winner won

See `references/agents.md` (comparator section) for details. This is optional and requires subagents.

## Description Optimization Workflow

### Step 1: Generate Trigger Eval Queries

Create 20 queries — mix of should-trigger (8-10) and should-not-trigger (8-10). Save as JSON:

```json
[
  {"query": "ok so my boss sent me this xlsx file and wants profit margin as a percentage...", "should_trigger": true},
  {"query": "write a fibonacci function in python", "should_trigger": false}
]
```

**Requirements:**
- Realistic, concrete, detailed (file paths, context, backstory)
- Mix of lengths, casual/formal, some with typos
- Should-trigger: different phrasings, implicit needs, uncommon use cases
- Should-not-trigger: near-misses, adjacent domains, ambiguous phrasing

Bad: `"Format this data"`, `"Extract text from PDF"`
Good: `"ok so my boss just sent me this xlsx file (its in my downloads, called something like 'Q4 sales final FINAL v2.xlsx') and she wants me to add a column that shows the profit margin as a percentage..."`

### Step 2: Review with User

Present using `assets/eval_review.html`:
1. Read template
2. Replace `__EVAL_DATA_PLACEHOLDER__` with JSON array
3. Replace `__SKILL_NAME_PLACEHOLDER__` and `__SKILL_DESCRIPTION_PLACEHOLDER__`
4. Write to temp file and open

User can edit, toggle, add/remove. Exports to `~/Downloads/eval_set.json`.

### Step 3: Run Optimization Loop

```bash
python -m scripts.run_loop \
  --eval-set <path-to-trigger-eval.json> \
  --skill-path <path-to-skill> \
  --model <model-id> \
  --max-iterations 5 \
  --verbose
```

**Process:**
- Splits eval set into 60% train / 40% held-out test
- Evaluates current description (3 runs per query for reliable trigger rate)
- Calls model to propose improvements based on failures
- Re-evaluates new descriptions on both train and test
- Iterates up to 5 times
- Selects `best_description` by test score (not train) to avoid overfitting

**How triggering works:** Skills appear in `available_skills` with name + description. The model decides whether to consult a skill based on that description. It only consults skills for tasks it can't easily handle on its own — simple one-step queries may not trigger even with perfect descriptions. Eval queries must be substantive enough that the model would benefit from a skill.

### Step 4: Apply Result

Update SKILL.md frontmatter with `best_description`. Show before/after and report scores.
