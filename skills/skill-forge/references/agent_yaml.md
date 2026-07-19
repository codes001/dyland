# agents/agent.yaml Reference

UI-facing metadata for skill lists and chips. This file controls how the skill appears in skill selection interfaces.

## Required Fields

### display_name
- **Type**: string
- **Description**: Human-readable name shown in skill lists and chips
- **Constraints**: Under 64 characters, plain text (no markdown)
- **Example**: `PDF Editor`, `BigQuery Helper`, `Frontend Builder`

### short_description
- **Type**: string
- **Description**: One-line summary for hover tooltips and compact lists
- **Constraints**: Under 120 characters
- **Example**: `Create, edit, and analyze PDF documents`, `Query BigQuery databases with schema-aware assistance`

### default_prompt
- **Type**: string
- **Description**: Suggested prompt shown when the user selects this skill
- **Constraints**: Under 200 characters, should be a concrete example prompt
- **Example**: `Help me create a professional report from this data`, `Query the sales table for Q4 revenue by region`

## Optional Fields

### icon
- **Type**: string (emoji or URL)
- **Description**: Visual identifier for the skill
- **Example**: `📄`, `🔍`, `https://example.com/icon.png`
- **Note**: Only include if the user explicitly provides an icon

### brand_color
- **Type**: string (hex color)
- **Description**: Accent color for skill chips
- **Example**: `#FF6B6B`, `#4ECDC4`
- **Note**: Only include if the user explicitly provides a color

### tags
- **Type**: array of strings
- **Description**: Categories for filtering and grouping
- **Example**: `["data", "analysis", "reporting"]`
- **Note**: Only include if the user explicitly provides tags

## Generation

Generate `display_name`, `short_description`, and `default_prompt` by reading the skill's SKILL.md and understanding its purpose. Then pass them as `--interface key=value` to `scripts/init_skill.py` or `scripts/generate_agent_yaml.py`.

Example:
```bash
python -m scripts.init_skill my-skill --path skills/public \
  --interface display_name="My Skill" \
  --interface short_description="Does useful things" \
  --interface default_prompt="Help me do a useful thing"
```

## Example agent.yaml

```yaml
display_name: PDF Editor
short_description: Create, edit, and analyze PDF documents with advanced features
default_prompt: Help me extract text from this PDF and format it as a markdown document
```

## Validation Rules

- `display_name` must not be empty
- `short_description` must not be empty
- `default_prompt` must not be empty
- No additional fields should be present unless explicitly provided by the user