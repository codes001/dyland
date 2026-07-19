# JSON Schemas

## evals.json

```json
{
  "skill_name": "example-skill",
  "evals": [
    {
      "id": 1,
      "prompt": "User's task prompt",
      "expected_output": "Description of expected result",
      "files": ["path/to/input.file"],
      "assertions": [
        {
          "name": "output_has_chart",
          "description": "The output contains a chart with axis labels",
          "type": "programmatic",
          "check": "python expression or script path"
        }
      ]
    }
  ]
}
```

Fields:
- `skill_name`: String, identifier for the skill being evaluated
- `evals`: Array of test cases
  - `id`: Unique integer identifier
  - `prompt`: The user prompt to test
  - `expected_output`: Human-readable description of expected result
  - `files`: Array of input file paths (optional)
  - `assertions`: Array of assertion objects (optional, can be added later)
    - `name`: Machine-readable identifier
    - `description`: Human-readable description (appears in viewer)
    - `type`: `"programmatic"` or `"subjective"`
    - `check`: For programmatic: script path or inline expression

## eval_metadata.json

```json
{
  "eval_id": 0,
  "eval_name": "descriptive-name-here",
  "prompt": "The user's task prompt",
  "assertions": [
    {
      "name": "has_axis_labels",
      "description": "Chart has both x and y axis labels"
    }
  ]
}
```

Fields:
- `eval_id`: Integer matching evals.json id
- `eval_name`: Descriptive string for directory naming and viewer display
- `prompt`: The exact prompt given to the subagent
- `assertions`: Array of assertion descriptors

## timing.json

```json
{
  "total_tokens": 84852,
  "duration_ms": 23332,
  "total_duration_seconds": 23.3
}
```

Fields:
- `total_tokens`: Integer, total tokens consumed
- `duration_ms`: Integer, wall-clock duration in milliseconds
- `total_duration_seconds`: Float, human-readable duration

## grading.json

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

**CRITICAL**: Must use exact field names `text`, `passed`, `evidence`. The viewer depends on these.

Fields:
- `expectations`: Array of graded assertions
  - `text`: String, the assertion description
  - `passed`: Boolean, whether the assertion passed
  - `evidence`: String, explanation or proof

## benchmark.json

```json
{
  "skill_name": "my-skill",
  "iteration": 1,
  "timestamp": "2026-05-05T22:00:00Z",
  "configurations": [
    {
      "name": "with_skill",
      "evals": [
        {
          "eval_id": 0,
          "eval_name": "descriptive-name",
          "pass_rate": 0.85,
          "total_tokens": 85000,
          "duration_ms": 25000
        }
      ],
      "aggregate": {
        "mean_pass_rate": 0.82,
        "stddev_pass_rate": 0.05,
        "mean_tokens": 84000,
        "stddev_tokens": 2000,
        "mean_duration_ms": 24000,
        "stddev_duration_ms": 1500
      }
    },
    {
      "name": "without_skill",
      "evals": [...],
      "aggregate": {...}
    }
  ],
  "deltas": {
    "pass_rate_delta": 0.15,
    "tokens_delta": -5000,
    "duration_delta": -2000
  },
  "analyst_observations": [
    "Assertion 'has_file' passes in both configurations — non-discriminating",
    "Eval-2 shows high variance in duration (stddev 45% of mean)"
  ]
}
```

Structure:
- `skill_name`: String
- `iteration`: Integer
- `timestamp`: ISO 8601 string
- `configurations`: Array, with_skill BEFORE baseline
  - `name`: "with_skill" or "without_skill" / "old_skill"
  - `evals`: Per-eval metrics
  - `aggregate`: Mean and stddev across evals
- `deltas`: with_skill minus baseline (positive = skill is better for pass_rate, negative = skill is better for tokens/duration)
- `analyst_observations`: Array of strings

## feedback.json

```json
{
  "reviews": [
    {
      "run_id": "eval-0-with_skill",
      "feedback": "the chart is missing axis labels",
      "timestamp": "2026-05-05T22:30:00Z"
    },
    {
      "run_id": "eval-1-with_skill",
      "feedback": "",
      "timestamp": "2026-05-05T22:31:00Z"
    }
  ],
  "status": "complete"
}
```

Fields:
- `reviews`: Array of feedback entries
  - `run_id`: String, format `<eval-name>-<config>`
  - `feedback`: String, empty if no issues
  - `timestamp`: ISO 8601 string
- `status`: "complete" or "in_progress"

## trigger_eval.json

```json
[
  {"query": "...", "should_trigger": true},
  {"query": "...", "should_trigger": false}
]
```

Array of objects:
- `query`: String, realistic user prompt
- `should_trigger`: Boolean, whether this skill should be consulted
