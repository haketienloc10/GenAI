---
name: review-code-diff
description: Review code changes and suggest improvements based on git diff.
version: 1.0.0
category: code_quality
tags: [git, diff, review, code-quality]

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
  type: markdown
  style: code_review
---

# Overview
This skill reviews the current code changes (from git diff) and returns actionable feedback: risks, bugs, design issues, and concrete improvements.

# Rules
* Prefer staged diff (`git diff --staged`)
* If staged diff is empty, fallback to unstaged diff
* Output in Markdown with clear sections and bullet points
* Focus on correctness, security, performance, readability, maintainability
* Provide concrete suggestions (what/why/how), avoid vague advice
* If diff is empty: respond that there are no changes to review

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

### Step: review_changes
```genai-step
id: review_changes
type: llm
model: gemini-2.5-flash
input_vars: [diff]
prompt: |
  You are a senior software engineer doing a PR review.

  IMPORTANT:
  - All output MUST be written in Vietnamese.
  - Use professional, technical Vietnamese terminology.

  Task:
  Review the following git diff and produce actionable feedback.

  Output format (Markdown):
  ## Summary
  - 1-3 bullets summarizing what changed and overall quality.

  ## Critical Issues (must-fix)
  - List only high severity issues: correctness bugs, security, data loss, crashes, race conditions.
  - For each item: **Issue** / **Why it matters** / **Suggested fix**

  ## Recommendations (should improve)
  - Design, readability, maintainability, performance, error handling, naming, tests.
  - For each item: **Suggestion** / **Rationale** / **Example** (when useful)

  ## Test Checklist
  - Bullet list of tests to run or add (unit/integration/e2e), edge cases.

  Rules:
  - If the diff is empty, output ONLY: "Không có thay đổi nào để review."
  - Do NOT invent code that isn't present.
  - Prefer referencing exact symbols/lines from the diff where possible.
  - Keep tone constructive and objective.
  - Avoid generic comments.
  - Be concise but precise.
  - If you need more context, ask at most 3 precise questions in a final section:
    ## Questions

  Git diff:
  {{diff}}
output_var: review
```

### Step: respond
```genai-step
id: respond
type: output
format: markdown
template: |
  {{review}}
```
