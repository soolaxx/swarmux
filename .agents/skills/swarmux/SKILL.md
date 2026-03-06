---
name: swarmux
description: Agent-first tmux swarm orchestration CLI. Use it to submit, start, inspect, steer, reconcile, and prune local coding tasks with machine-readable output and strict input validation.
---

# Swarmux

Use `swarmux` as the control plane for local coding tasks.

Setup reference: `reference/setup.md`

## Invariants

- Prefer `--output json` for all machine consumption.
- Prefer raw payload input with `--json` or `--json-file` for mutating commands.
- Use `--dry-run` before real mutations when validating a payload or workflow.
- Treat `schema` as the source of truth for command shape.
- Never bypass `swarmux` with ad hoc tmux or git worktree commands unless you are repairing a broken session.

## Workflow

1. Run `swarmux doctor`.
2. Run `swarmux init`.
3. Inspect command shapes with `swarmux --output json schema`.
4. Submit tasks with raw JSON payloads.
5. Use `start` or `delegate` to launch work.
6. Use `logs`, `show`, `list`, `popup`, and `reconcile` for supervision.
7. Use `stop`, `done`, `fail`, and `prune` for explicit control.

## Safety

- Inputs are validated defensively because agents hallucinate paths and identifiers.
- `logs` is sanitized by default; use `--raw` only when needed.
- `prune` is dry-run by default. Add `--apply` only when intentional.
- The beads backend is optional. Default backend is `files`.
