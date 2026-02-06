---
name: auto-commit-msg
description: Generate a Conventional Commit message based on git diff.
version: 1.0.0
category: git
tags: [git, commit, diff]

entrypoint: workflow
workflow_version: 1

capabilities:
  requires_repo: true
  supports_interactive: true

permissions:
  run_commands: true
  allowed_runners: [bash]
  allowed_paths:
    - scripts/
  network_access: true
  write_access: false

response_format:
  type: plain_text
  style: conventional_commit
---

# Overview
This skill generates a clean Conventional Commit message by analyzing the current git diff.

# Rules
- Use Conventional Commits format: `<type>(<scope>): <message>`
- Prefer staged diff (`git diff --staged`)
- If staged diff is empty, fallback to unstaged diff
- Keep summary <= 72 characters

# Workflow

### Step: get_staged_diff
```genai-step
id: get_staged_diff
type: command
runner: bash
cmd: git diff --staged
output_var: diff
```

### Step: fallback_unstaged

```genai-step
id: fallback_unstaged
type: command
runner: bash
cmd: git diff
output_var: diff
if: "{{diff}} == ''"
```

### Step: generate_commit_message

```genai-step
id: generate_commit_message
type: llm
model: gemini-2.5-flash
input_vars: [diff]
prompt: |
  You are a senior software engineer.
  Generate ONE git commit message using Conventional Commits.

  Rules:
  - Output ONLY the commit message line.
  - Keep it under 72 characters.
  - Use type: feat, fix, refactor, docs, test, chore.
  - Add scope only if obvious.

  Git diff:
  {{diff}}
output_var: commit_message
```

### Step: respond

```genai-step
id: respond
type: output
format: plain_text
template: |
  {{commit_message}}
```
