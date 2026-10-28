// Packages the extension as a .vsix for the platform of this computer (the bundled
// tok-lsp is a native program): tok-PLATFORM-VERSION.vsix, for example
// tok-win32-x64-1.0.0.vsix. Nothing is published. The code is one bundled file
// (scripts/bundle.mjs), so no node_modules go into the package.

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(dirname(fileURLToPath(import.meta.url)));
const { version } = JSON.parse(readFileSync(join(here, "package.json"), "utf8"));
const platform = process.platform === "win32" ? "win32" : process.platform === "darwin" ? "darwin" : "linux";
const target = `${platform}-${process.arch}`;
const out = `tok-${target}-${version}.vsix`;
const vsce = join(here, "node_modules", "@vscode", "vsce", "vsce");
execFileSync(process.execPath, [vsce, "package", "--no-dependencies", "--target", target, "--out", out], { cwd: here, stdio: "inherit" });
console.log(out);
