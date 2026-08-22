#!/usr/bin/env node
/**
 * Generates clean, categorized release notes for a given tag by
 * reading git log between the previous tag and this one, grouping
 * commits by Conventional Commit type. No dependencies — plain git
 * + node so it runs the same in CI and locally.
 *
 * Usage:
 *   node .github/scripts/generate-release-notes.mjs v0.2.0 > release-notes.md
 *
 * If the tag doesn't exist yet locally (workflow_dispatch before the
 * tag is pushed), pass HEAD as the upper bound instead:
 *   node .github/scripts/generate-release-notes.mjs v0.2.0 --upto=HEAD
 */
import { execSync } from "node:child_process";

const args = process.argv.slice(2);
const tag = args.find((a) => !a.startsWith("--"));
const uptoArg = args.find((a) => a.startsWith("--upto="));
const upto = uptoArg ? uptoArg.split("=")[1] : tag;

if (!tag) {
  console.error("Usage: generate-release-notes.mjs <tag> [--upto=<ref>]");
  process.exit(1);
}

function sh(cmd) {
  return execSync(cmd, { encoding: "utf-8" }).trim();
}

function previousTag() {
  try {
    // First tag reachable before `upto`, excluding `upto`/`tag` itself.
    const tags = sh(`git tag --sort=-creatordate --merged ${upto}`)
      .split("\n")
      .filter(Boolean)
      .filter((t) => t !== tag);
    return tags[0] || null;
  } catch {
    return null;
  }
}

const prev = previousTag();
const range = prev ? `${prev}..${upto}` : upto;

let log;
try {
  log = sh(`git log ${range} --pretty=format:%H%x01%s%x01%b%x02`);
} catch (e) {
  console.error(`Failed to read git log for range ${range}:`, e.message);
  process.exit(1);
}

const commits = log
  .split("\x02")
  .map((s) => s.trim())
  .filter(Boolean)
  .map((entry) => {
    const [hash, subject, body] = entry.split("\x01");
    return { hash: hash?.slice(0, 7), subject: subject?.trim(), body: body?.trim() };
  })
  .filter((c) => c.subject && !/^Merge (pull request|branch)/i.test(c.subject));

// Conventional-commit type -> section heading. Anything that doesn't
// match a known prefix goes under "Other changes" rather than being
// dropped, so nothing silently disappears from the notes.
const SECTIONS = [
  { prefix: "feat", heading: "✨ Features" },
  { prefix: "fix", heading: "🐛 Fixes" },
  { prefix: "security", heading: "🔒 Security" },
  { prefix: "perf", heading: "⚡ Performance" },
  { prefix: "docs", heading: "📝 Documentation" },
  { prefix: "refactor", heading: "🧹 Refactoring" },
  { prefix: "test", heading: "✅ Tests" },
  { prefix: "chore", heading: "🔧 Chores" },
];

const buckets = new Map(SECTIONS.map((s) => [s.prefix, []]));
const other = [];
const breaking = [];

const CONVENTIONAL = /^(\w+)(\([^)]*\))?(!)?:\s*(.+)$/;

for (const commit of commits) {
  const match = commit.subject.match(CONVENTIONAL);
  const isBreaking = commit.body?.includes("BREAKING CHANGE") || match?.[3] === "!";

  if (isBreaking) {
    breaking.push({ ...commit, cleaned: match ? match[4] : commit.subject });
  }

  if (match && buckets.has(match[1].toLowerCase())) {
    buckets.get(match[1].toLowerCase()).push({ ...commit, cleaned: match[4] });
  } else if (!isBreaking) {
    other.push(commit);
  }
}

const lines = [];
lines.push(`## ${tag}`);
lines.push("");

if (breaking.length) {
  lines.push("### ⚠️ Breaking changes");
  for (const c of breaking) lines.push(`- ${c.cleaned} (${c.hash})`);
  lines.push("");
}

for (const { prefix, heading } of SECTIONS) {
  const items = buckets.get(prefix);
  if (!items.length) continue;
  lines.push(`### ${heading}`);
  for (const c of items) lines.push(`- ${c.cleaned} (${c.hash})`);
  lines.push("");
}

if (other.length) {
  lines.push("### Other changes");
  for (const c of other) lines.push(`- ${c.subject} (${c.hash})`);
  lines.push("");
}

if (!commits.length) {
  lines.push("_No changes recorded since the previous tag._");
  lines.push("");
}

// GitHub-native compare link — works once both refs exist on the remote.
try {
  const remoteUrl = sh("git config --get remote.origin.url")
    .replace(/\.git$/, "")
    .replace(/^git@github\.com:/, "https://github.com/");
  if (prev) {
    lines.push(`**Full changelog:** ${remoteUrl}/compare/${prev}...${tag}`);
  } else {
    lines.push(`**Full changelog:** ${remoteUrl}/commits/${tag}`);
  }
} catch {
  // No remote configured (e.g. local test run) — skip the link.
}

console.log(lines.join("\n"));
