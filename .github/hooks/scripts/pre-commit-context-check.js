#!/usr/bin/env node

/*
  PreToolUse hook: blocks commit commands if staged source file changes
  do not include an updated staged context.md.
*/

import { execSync } from "node:child_process";

function readStdin() {
  return new Promise((resolve) => {
    let data = "";
    process.stdin.setEncoding("utf8");
    process.stdin.on("data", (chunk) => {
      data += chunk;
    });
    process.stdin.on("end", () => resolve(data));
    process.stdin.on("error", () => resolve(""));
  });
}

function parseJson(text) {
  if (!text || !text.trim()) {
    return {};
  }

  try {
    return JSON.parse(text);
  } catch {
    return {};
  }
}

function safeString(value) {
  return typeof value === "string" ? value : "";
}

function getCommand(payload) {
  const directCandidates = [
    payload.command,
    payload.toolInput,
    payload.tool_input,
    payload.input,
  ];

  for (const candidate of directCandidates) {
    if (typeof candidate === "string" && candidate.trim()) {
      return candidate;
    }
  }

  const nestedCandidates = [
    payload.toolInput && payload.toolInput.command,
    payload.tool_input && payload.tool_input.command,
    payload.input && payload.input.command,
    payload.arguments && payload.arguments.command,
  ];

  for (const candidate of nestedCandidates) {
    if (typeof candidate === "string" && candidate.trim()) {
      return candidate;
    }
  }

  return "";
}

function emitDecision(permissionDecision, reason) {
  const output = {
    hookSpecificOutput: {
      hookEventName: "PreToolUse",
      permissionDecision,
      permissionDecisionReason: reason,
    },
  };

  process.stdout.write(`${JSON.stringify(output)}\n`);
}

function runGit(command) {
  return execSync(command, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
  }).trim();
}

function isSourcePath(path) {
  return (
    path.startsWith("src/") ||
    path.startsWith("src-tauri/src/") ||
    path === "package.json" ||
    path === "src-tauri/Cargo.toml" ||
    path === "src-tauri/tauri.conf.json"
  );
}

async function main() {
  const rawInput = await readStdin();
  const payload = parseJson(rawInput);

  const command = safeString(getCommand(payload));
  const normalized = command.toLowerCase();

  if (!/\bgit\s+commit\b/.test(normalized)) {
    emitDecision("allow", "Non-commit command; pre-commit context check skipped.");
    return;
  }

  try {
    const insideRepo = runGit("git rev-parse --is-inside-work-tree");
    if (insideRepo !== "true") {
      emitDecision("allow", "Not inside a git repository; pre-commit context check skipped.");
      return;
    }
  } catch {
    emitDecision("allow", "Git repository check failed; pre-commit context check skipped.");
    return;
  }

  let staged = [];
  try {
    const stagedRaw = runGit("git diff --cached --name-only --diff-filter=ACMR");
    staged = stagedRaw
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean);
  } catch {
    emitDecision("allow", "Unable to read staged files; pre-commit context check skipped.");
    return;
  }

  if (staged.length === 0) {
    emitDecision("allow", "No staged files detected.");
    return;
  }

  const sourceChanged = staged.some(isSourcePath);
  if (!sourceChanged) {
    emitDecision("allow", "No staged source/config changes detected.");
    return;
  }

  const contextStaged = staged.includes("context.md");
  if (contextStaged) {
    emitDecision("allow", "context.md is staged with source/config changes.");
    return;
  }

  emitDecision(
    "deny",
    "Commit blocked: staged source/config changes require an updated staged context.md."
  );
}

main().catch(() => {
  emitDecision("allow", "Hook execution failed unexpectedly; commit guard skipped.");
});