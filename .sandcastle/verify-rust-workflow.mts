import {
  createWorktree,
  type BindMountCreateOptions,
  type BindMountSandboxHandle,
} from "@ai-hero/sandcastle";
import { docker } from "@ai-hero/sandcastle/sandboxes/docker";

const sandboxWorkspace = "/home/agent/workspace";
const verificationBranch = "sandcastle/verify-rust-workflow";

const setupCommands = ["npm install"];
const workflowCommands = [
  "rustup --version",
  "cargo fmt",
  "npm run typecheck",
  "npm run test",
];

type DockerBindMountProvider = {
  create(options: BindMountCreateOptions): Promise<BindMountSandboxHandle>;
};

const logCommands = (heading: string, commands: readonly string[]) => {
  console.log(heading);
  for (const command of commands) {
    console.log(`- ${command}`);
  }
};

const createDockerSandbox = (options: BindMountCreateOptions) =>
  (docker() as unknown as DockerBindMountProvider).create(options);

const runCommand = async (
  sandbox: BindMountSandboxHandle,
  command: string,
) => {
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
console.log("Host command: npm run sandcastle:verify-rust-workflow");
logCommands("Sandbox setup hook:", setupCommands);
logCommands("Sandbox workflow commands:", workflowCommands);
console.log("Runtime Rust installer commands are intentionally forbidden.");

const worktree = await createWorktree({
  branchStrategy: { type: "branch", branch: verificationBranch },
});

let sandbox: BindMountSandboxHandle | undefined;

try {
  const activeSandbox = await createDockerSandbox({
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
  sandbox = activeSandbox;

  for (const command of setupCommands) {
    await runCommand(activeSandbox, command);
  }

  for (const command of workflowCommands) {
    await runCommand(activeSandbox, command);
  }

  console.log("\nSandcastle Rust workflow verification passed.");
} finally {
  await sandbox?.close();
  await worktree.close();
}
