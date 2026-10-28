// Copies the tok-lsp program of the repository build into server/ (built with cargo when it
// is missing) and the licence of tok next to the manifest, for packaging.

import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(dirname(fileURLToPath(import.meta.url)));
const root = join(here, "..", "..");
const exe = process.platform === "win32" ? "tok-lsp.exe" : "tok-lsp";
const built = join(root, "rust", "target", "release", exe);
if (!existsSync(built)) {
  execFileSync("cargo", ["build", "--release", "-p", "tok-lsp"], { cwd: join(root, "rust"), stdio: "inherit" });
}
mkdirSync(join(here, "server"), { recursive: true });
copyFileSync(built, join(here, "server", exe));
copyFileSync(join(root, "LICENSE"), join(here, "LICENSE.txt"));
console.log(`server/${exe} and LICENSE.txt copied`);
