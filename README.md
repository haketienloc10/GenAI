# genai

`genai` is a Rust runtime for skill-based agent workflows.

## Features

- Scan `skills/*/SKILL.md`
- Parse YAML frontmatter + `genai-step` blocks
- Validate skill metadata and workflow constraints
- Execute workflow steps (`command`, `llm`, `output`)
- Minimal template resolution (`{{var}}`)
- Basic conditional evaluation (`if: "{{var}} == ''"`)
- Skill selection via LLM JSON response with keyword fallback
- CLI with `list`, `run`, `run-skill`

## CLI

```bash
genai list --skills-dir ./skills
genai run "generate commit message" --skills-dir ./skills
genai run-skill auto-commit-msg "generate commit" --skills-dir ./skills
```

## Skill format

Each skill must have `SKILL.md` with:

1. YAML frontmatter between `---` and `---`
2. Markdown body
3. One or more workflow blocks:

````md
```genai-step
id: step_id
type: command
runner: bash
cmd: echo hello
output_var: out
```
````

See `skills/auto-commit-msg/SKILL.md` for a complete example.
