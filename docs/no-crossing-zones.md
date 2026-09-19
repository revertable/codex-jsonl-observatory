# No-Crossing Zones

## Purpose

This document defines project boundaries that agents may observe but must not cross without explicit approval.

Observation is allowed.
Modification is not allowed unless the user explicitly selects the target as current work.

---

## Current Repository Boundary

The current work target is this repository:

```text
codex-session-observatory
```

Agents may create, edit, and verify files inside this repository only when the task route allows execution.

Files outside this repository are outside the default execution boundary.

---

## Original Kotlin/Swing Repository

The original Kotlin/Swing implementation is reference material.

Original project:

```text
codex-chat-viewer
```

It may be inspected to understand:

* parser behavior
* data models
* rendered entry kinds
* search behavior
* filter behavior
* collapse behavior
* view styles
* Markdown export behavior
* packaging behavior
* tests worth porting

It must not be modified as part of normal work in this repository.

Crossing rule:

```text
Observe: allowed
Modify: not allowed
Copy concepts: allowed
Copy private/local state: not allowed
Execute commands inside it: only for read-only inspection unless explicitly approved
```

If a task requires modifying the original Kotlin/Swing repository, stop and ask for explicit approval.

---

## Local Filesystem Boundary

Agents must not modify files outside the current repository unless the user explicitly selects that external path as the current work target.

This includes:

* sibling repositories
* parent directories
* local tool configuration
* user home directories
* downloaded logs
* real Codex session logs
* private notes
* generated files outside this repository

---

## Sample Data Boundary

Real private Codex logs must not be committed.

Only sanitized sample JSONL files may be placed under:

```text
sample-data/
```

Sample data must not include:

* private prompts
* secrets or tokens
* real local filesystem paths
* private project names
* customer or company data
* personal data
* machine-specific information

---

## Stop Rule

If the agent is unsure whether a file, directory, repository, or command target is inside the allowed boundary, it must stop before execution.

Stop response:

```text
Observed:
- ...

No-Crossing Concern:
- ...

Decision Needed:
- ...
```

The agent must not resolve boundary uncertainty by assumption.
