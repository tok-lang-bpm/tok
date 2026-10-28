// Runs test/vscode/index.js inside VS Code with this folder as the extension under
// development, in a fresh user and extensions folder. It opens a VS Code window, so it
// needs a desktop; it is not part of npm test. The VS Code program is found from the
// variable VSCODE_CLI or the `code` command on the PATH.
//
//   npm run test:vscode

import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, realpathSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(dirname(fileURLToPath(import.meta.url)));

function findCli() {
  if (process.env.VSCODE_CLI) return process.env.VSCODE_CLI;
  const names = process.platform === "win32" ? ["code.cmd", "code"] : ["code"];
  for (const dir of (process.env.PATH ?? "").split(process.platform === "win32" ? ";" : ":")) {
    for (const n of names) if (dir && existsSync(join(dir, n))) return join(dir, n);
  }
  return null;
}

const cli = findCli();
if (!cli) {
  console.error("test:vscode: the VS Code command line was not found (set VSCODE_CLI)");
  process.exit(1);
}
// the program next to the command line: bin/code(.cmd) → Code.exe on Windows; on Linux and
// macOS the command line itself starts the program
const real = realpathSync(cli);
const program = process.platform === "win32" ? join(dirname(dirname(real)), "Code.exe") : real;
const work = mkdtempSync(join(tmpdir(), "tok-vscode-host-"));
const r = spawnSync(
  program,
  [
    "--extensionDevelopmentPath=" + here,
    "--extensionTestsPath=" + join(here, "test", "vscode", "index.js"),
    "--user-data-dir=" + join(work, "user"),
    "--extensions-dir=" + join(work, "extensions"),
    "--disable-workspace-trust",
    "--skip-welcome",
    "--skip-release-notes",
    "--new-window",
  ],
  { stdio: "inherit", timeout: 180000, env: { ...process.env, ELECTRON_RUN_AS_NODE: undefined } },
);
try {
  rmSync(work, { recursive: true, force: true, maxRetries: 5, retryDelay: 500 });
} catch {
  // VS Code may still hold files of its user folder; the system cleans the temporary folder
}
if (r.status !== 0) {
  console.error(`test:vscode: failed (exit status ${r.status})`);
  process.exit(1);
}
console.log("test:vscode: passed");
