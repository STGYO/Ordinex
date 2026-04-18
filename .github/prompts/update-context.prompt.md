---
description: "Use when: refreshing context.md after larger refactors, architecture changes, or multi-file updates"
name: "Update Context Snapshot"
argument-hint: "Optional: summarize recent changes to focus the refresh"
agent: "agent"
---
Refresh the repository context snapshot at [context.md](../../context.md).

Task:
- Re-scan the workspace source and config files.
- Keep generated directories excluded unless explicitly requested: `src-tauri/target`, `node_modules`, `dist`, `.git`.
- Update [context.md](../../context.md) so it matches the current codebase state.

Minimum updates required:
- Generated timestamp and scoped file count.
- Commands, stack, and architecture summaries if they changed.
- File Inventory entries for added, removed, renamed, or materially changed files.

Output requirements:
- Keep the snapshot concise and accurate.
- End with a short bullet list of what changed in [context.md](../../context.md).