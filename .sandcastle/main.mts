// Parallel Planner with Review — four-phase orchestration loop
//
// This template drives a multi-phase workflow:
//   Phase 1 (Plan):             An opus agent analyzes open issues, builds a
//                               dependency graph, and outputs a <plan> JSON
//                               listing unblocked issues with branch names.
//   Phase 2 (Execute + Review): For each issue, a sandbox is created via
//                               createSandbox(). The implementer runs first
//                               (100 iterations). If it produces commits, a
//                               reviewer runs in the same sandbox on the same
//                               branch (1 iteration). All issue pipelines run
//                               concurrently via Promise.allSettled().
//   Phase 3 (Merge):            A single agent merges all completed branches
//                               into the current branch.
//
// The outer loop repeats up to MAX_ITERATIONS times so that newly unblocked
// issues are picked up after each round of merges.
//
// Usage:
//   npx tsx .sandcastle/main.mts
// Or add to package.json:
//   "scripts": { "sandcastle": "npx tsx .sandcastle/main.mts" }

import * as sandcastle from "@ai-hero/sandcastle";
import { docker } from "@ai-hero/sandcastle/sandboxes/docker";
import { config as loadEnvFile } from "dotenv";
import { z } from "zod";
import os from "node:os";
import path from "node:path";

loadEnvFile({ path: ".sandcastle/.env", quiet: true });

const hostCodexHome = path.join(os.homedir(), ".codex");
const sandboxCodexMount = "/mnt/host-codex";
const sandboxCodexHome = "/home/agent/.codex";

// E2E testing needs real Google accounts inside the sandbox, but the
// sandbox has no access to the host OS keychain that `goog` normally reads
// from. Run `goog auth export --out .sandcastle/secrets/token.json` once on
// the host to export every authorized account, and drop a copy of
// ~/.config/goog/config.toml at .sandcastle/secrets/config.toml (see
// docs/agents/e2e-testing.md), to make E2E testing available to the
// implementer/reviewer sandbox. Both files are optional -- if absent, the
// copy step below is a no-op and `goog` commands in the sandbox fail with
// "not logged in" instead of silently working.
const hostE2eSecretsDir = path.join(process.cwd(), ".sandcastle", "secrets");
const sandboxE2eSecretsMount = "/mnt/e2e-secrets";
const sandboxGoogConfigHome = "/home/agent/.config/goog";
const sandboxGoogTokenFile = "/home/agent/.config/goog/e2e-token.json";

// The planner emits its plan as JSON inside <plan> tags; Output.object extracts
// and validates it against this schema. We use Zod here, but any Standard
// Schema validator works just as well — Valibot, ArkType, etc. See
// https://standardschema.dev.
const planSchema = z.object({
  issues: z.array(
    z.object({ id: z.string(), title: z.string(), branch: z.string() }),
  ),
});

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

// Maximum number of plan→execute→merge cycles before stopping.
// Raise this if your backlog is large; lower it for a quick smoke-test run.
const MAX_ITERATIONS = 10;

// Hooks run inside the sandbox before the agent starts each iteration.
// npm install ensures the sandbox always has fresh dependencies.
const hooks = {
  sandbox: {
    onSandboxReady: [
      {
        command: [
          `mkdir -p "${sandboxCodexHome}"`,
          `test -f "${sandboxCodexMount}/auth.json"`,
          `cp "${sandboxCodexMount}/auth.json" "${sandboxCodexHome}/auth.json"`,
          `if [ -f "${sandboxCodexMount}/config.toml" ]; then cp "${sandboxCodexMount}/config.toml" "${sandboxCodexHome}/config.toml"; fi`,
          // Copy in the exported goog E2E credentials, if the human has set them up.
          // See docs/agents/e2e-testing.md for how to create these two files.
          `mkdir -p "${sandboxGoogConfigHome}"`,
          `if [ -f "${sandboxE2eSecretsMount}/config.toml" ]; then cp "${sandboxE2eSecretsMount}/config.toml" "${sandboxGoogConfigHome}/config.toml"; fi`,
          `if [ -f "${sandboxE2eSecretsMount}/token.json" ]; then cp "${sandboxE2eSecretsMount}/token.json" "${sandboxGoogTokenFile}"; fi`,
        ].join(" && "),
      },
      { command: "npm install" },
    ],
  },
};

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------

for (let iteration = 1; iteration <= MAX_ITERATIONS; iteration++) {
  console.log(`\n=== Iteration ${iteration}/${MAX_ITERATIONS} ===\n`);

  // -------------------------------------------------------------------------
  // Phase 1: Plan
  //
  // The planning agent (opus, for deeper reasoning) reads the open issue list,
  // builds a dependency graph, and selects the issues that can be worked in
  // parallel right now (i.e., no blocking dependencies on other open issues).
  //
  // It outputs a <plan> JSON block — Output.object parses and validates it.
  // -------------------------------------------------------------------------
  const plan = await sandcastle.run({
    hooks,
    sandbox: docker({
      env: {
        CODEX_HOME: sandboxCodexHome,
        GH_TOKEN: process.env.GH_TOKEN ?? "",
      },
      mounts: [
        {
          hostPath: hostCodexHome,
          sandboxPath: sandboxCodexMount,
          readonly: true,
        },
      ],
    }),
    name: "planner",
    // One iteration is enough: the planner just needs to read and reason,
    // not write code. (Structured output requires maxIterations: 1.)
    maxIterations: 1,
    // Opus for planning: dependency analysis benefits from deeper reasoning.
    agent: sandcastle.codex("gpt-5.5"),
    promptFile: "./.sandcastle/plan-prompt.md",
    // Extract and validate the <plan> JSON into a typed object. Throws
    // StructuredOutputError if the tag is missing, the JSON is malformed, or
    // validation fails — which aborts the loop.
    output: sandcastle.Output.object({ tag: "plan", schema: planSchema }),
  });

  const issues = plan.output.issues;

  if (issues.length === 0) {
    // No unblocked work — either everything is done or everything is blocked.
    console.log("No unblocked issues to work on. Exiting.");
    break;
  }

  console.log(
    `Planning complete. ${issues.length} issue(s) to work in parallel:`,
  );
  for (const issue of issues) {
    console.log(`  ${issue.id}: ${issue.title} → ${issue.branch}`);
  }

  // -------------------------------------------------------------------------
  // Phase 2: Execute + Review
  //
  // For each issue, create a sandbox via createSandbox() so the implementer
  // and reviewer share the same sandbox instance per branch. The implementer
  // runs first; if it produces commits, the reviewer runs in the same sandbox.
  //
  // Promise.allSettled means one failing pipeline doesn't cancel the others.
  // -------------------------------------------------------------------------

  const settled = await Promise.allSettled(
    issues.map(async (issue) => {
      const sandbox = await sandcastle.createSandbox({
        branch: issue.branch,
        sandbox: docker({
          env: {
            CODEX_HOME: sandboxCodexHome,
            GH_TOKEN: process.env.GH_TOKEN ?? "",
            // Harmless if no secrets are set up: goog reports "not logged
            // in" instead of finding a keychain that doesn't exist in this
            // sandbox. See docs/agents/e2e-testing.md.
            GOOG_TOKEN_FILE: sandboxGoogTokenFile,
          },
          mounts: [
            {
              hostPath: hostCodexHome,
              sandboxPath: sandboxCodexMount,
              readonly: true,
            },
            {
              hostPath: hostE2eSecretsDir,
              sandboxPath: sandboxE2eSecretsMount,
              readonly: true,
            },
          ],
        }),
        hooks,
      });

      try {
        // Run the implementer
        const implement = await sandbox.run({
          name: "implementer",
          maxIterations: 100,
          agent: sandcastle.codex("gpt-5.5"),
          promptFile: "./.sandcastle/implement-prompt.md",
          promptArgs: {
            TASK_ID: issue.id,
            ISSUE_TITLE: issue.title,
            BRANCH: issue.branch,
          },
        });

        // Only review if the implementer produced commits
        if (implement.commits.length > 0) {
          const review = await sandbox.run({
            name: "reviewer",
            maxIterations: 1,
            agent: sandcastle.codex("gpt-5.5"),
            promptFile: "./.sandcastle/review-prompt.md",
            promptArgs: {
              TASK_ID: issue.id,
              BRANCH: issue.branch,
            },
          });

          // Merge commits from both runs so the merge phase sees all of them.
          // Each sandbox.run() only returns commits from its own run.
          return {
            ...review,
            commits: [...implement.commits, ...review.commits],
          };
        }

        return implement;
      } finally {
        await sandbox.close();
      }
    }),
  );

  // Log any agents that threw (network error, sandbox crash, etc.).
  for (const [i, outcome] of settled.entries()) {
    if (outcome.status === "rejected") {
      console.error(
        `  ✗ ${issues[i]!.id} (${issues[i]!.branch}) failed: ${outcome.reason}`,
      );
    }
  }

  // Only pass branches that actually produced commits to the merge phase.
  // An agent that ran successfully but made no commits has nothing to merge.
  const completedIssues = settled
    .map((outcome, i) => ({ outcome, issue: issues[i]! }))
    .filter(
      (entry) =>
        entry.outcome.status === "fulfilled" &&
        entry.outcome.value.commits.length > 0,
    )
    .map((entry) => entry.issue);

  const completedBranches = completedIssues.map((i) => i.branch);

  console.log(
    `\nExecution complete. ${completedBranches.length} branch(es) with commits:`,
  );
  for (const branch of completedBranches) {
    console.log(`  ${branch}`);
  }

  if (completedBranches.length === 0) {
    // All agents ran but none made commits — nothing to merge this cycle.
    console.log("No commits produced. Nothing to merge.");
    continue;
  }

  // -------------------------------------------------------------------------
  // Phase 3: Merge
  //
  // One agent merges all completed branches into the current branch,
  // resolving any conflicts and running tests to confirm everything works.
  //
  // The {{BRANCHES}} and {{ISSUES}} prompt arguments are lists that the agent
  // uses to know which branches to merge and which issues to close.
  // -------------------------------------------------------------------------
  await sandcastle.run({
    hooks,
    sandbox: docker({
      env: {
        CODEX_HOME: sandboxCodexHome,
        GH_TOKEN: process.env.GH_TOKEN ?? "",
      },
      mounts: [
        {
          hostPath: hostCodexHome,
          sandboxPath: sandboxCodexMount,
          readonly: true,
        },
      ],
    }),
    name: "merger",
    maxIterations: 1,
    agent: sandcastle.codex("gpt-5.5"),
    promptFile: "./.sandcastle/merge-prompt.md",
    promptArgs: {
      // A markdown list of branch names, one per line.
      BRANCHES: completedBranches.map((b) => `- ${b}`).join("\n"),
      // A markdown list of issue IDs and titles, one per line.
      ISSUES: completedIssues.map((i) => `- ${i.id}: ${i.title}`).join("\n"),
    },
  });

  console.log("\nBranches merged.");
}

console.log("\nAll done.");
