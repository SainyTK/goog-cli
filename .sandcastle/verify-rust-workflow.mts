import { createWorktree } from "@ai-hero/sandcastle";
import { docker } from "@ai-hero/sandcastle/sandboxes/docker";

const sandboxWorkspace = "/home/agent/workspace";
const verificationBranch = "sandcastle/verify-rust-workflow";

const setupCommands = [{ command: "npm install" }];
const workflowCommands = [
  "rustup --version",
  "cargo fmt",
  "npm run typecheck",
  "npm run test",
];

type ExecResult = {
  stdout: string;
  stderr: string;
  exitCode: number;
};

type ExecHandle = {
  close(): Promise<void>;
  exec(
    command: string,
    options?: { cwd?: string; onLine?: (line: string) => void },
  ): Promise<ExecResult>;
};

type BindMountProvider = {
  create(options: {
    worktreePath: string;
    hostRepoPath: string;
    mounts: Array<{ hostPath: string; sandboxPath: string; readonly?: boolean }>;
    env: Record<string, string>;
  }): Promise<ExecHandle>;
};

const runCommand = async (sandbox: ExecHandle, command: string) => {
  console.log(`\n$ ${command}`);
  const result = await sandbox.exec(command, {
    cwd: sandboxWorkspace,
    onLine: (line) => console.log(line),
  });

  if (result.exitCode !== 0) {
    if (result.stderr.trim().length > 0) {
      console.error(result.stderr);
    }
    throw new Error(`Command failed with exit code ${result.exitCode}: ${command}`);
  }
};

console.log("Sandcastle Rust workflow verifier");
console.log(`Host command: npm run sandcastle:verify-rust-workflow`);
console.log("Sandbox setup hook:");
for (const { command } of setupCommands) {
  console.log(`- ${command}`);
}
console.log("Sandbox workflow commands:");
for (const command of workflowCommands) {
  console.log(`- ${command}`);
}
console.log(
  "Runtime Rust installer commands are intentionally forbidden.",
);

const worktree = await createWorktree({
  branchStrategy: { type: "branch", branch: verificationBranch },
});

let sandbox: ExecHandle | undefined;

try {
  sandbox = await (docker() as unknown as BindMountProvider).create({
    worktreePath: worktree.worktreePath,
    hostRepoPath: process.cwd(),
    mounts: [
      {
        hostPath: worktree.worktreePath,
        sandboxPath: sandboxWorkspace,
      },
    ],
    env: {},
  });

  for (const { command } of setupCommands) {
    await runCommand(sandbox, command);
  }

  for (const command of workflowCommands) {
    await runCommand(sandbox, command);
  }

  console.log("\nSandcastle Rust workflow verification passed.");
} finally {
  await sandbox?.close();
  await worktree.close();
}
