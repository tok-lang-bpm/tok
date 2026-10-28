// Bundles the extension and the language client into one file, out/extension.js, with
// esbuild (the module `vscode` is provided by VS Code). The TypeScript compiler only
// checks the types (npm run build runs both).

import { build } from "esbuild";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(dirname(fileURLToPath(import.meta.url)));
await build({
  entryPoints: [join(here, "src", "extension.ts")],
  outfile: join(here, "out", "extension.js"),
  bundle: true,
  platform: "node",
  format: "cjs",
  target: "node20",
  external: ["vscode"],
  minify: true,
  sourcemap: false,
  legalComments: "inline",
  logLevel: "warning",
});
console.log("out/extension.js bundled");
