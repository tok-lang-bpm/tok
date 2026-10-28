// The grammar over every example and conformance document of the repository: each
// statement keyword is highlighted as a keyword, comment lines as comments, table rows as
// rows, and no string runs over the end of its line.

import { readFileSync, readdirSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const textmate = require("vscode-textmate");
const oniguruma = require("vscode-oniguruma");

const here = dirname(dirname(fileURLToPath(import.meta.url)));
const root = join(here, "..", "..");

const wasm = readFileSync(require.resolve("vscode-oniguruma/release/onig.wasm")).buffer;
await oniguruma.loadWASM(wasm);
const registry = new textmate.Registry({
  onigLib: Promise.resolve({
    createOnigScanner: (patterns) => new oniguruma.OnigScanner(patterns),
    createOnigString: (s) => new oniguruma.OnigString(s),
  }),
  loadGrammar: async () =>
    textmate.parseRawGrammar(readFileSync(join(here, "syntaxes", "tok.tmLanguage.json"), "utf8"), "tok.tmLanguage.json"),
});
const grammar = await registry.loadGrammar("source.tok");

const keywordScopes = ["storage.type.tok", "keyword.control.tok", "keyword.other.header.tok"];
// the keywords of the grammar (the error cases of the corpus also use words that are not)
const raw = JSON.parse(readFileSync(join(here, "syntaxes", "tok.tmLanguage.json"), "utf8"));
const keywords = new Set(["tok", "flow", "message", "reads", "writes"]);
for (const k of raw.repository.declaration.match.match(/\(([a-z|]+)\)/)[1].split("|")) keywords.add(k);
const files = [];
for (const dir of ["examples", join("conformance", "cases")]) {
  for (const name of readdirSync(join(root, dir)).sort()) {
    if (name.endsWith(".tok")) files.push(join(dir, name));
  }
}

let failures = 0;
let lines = 0;
for (const file of files) {
  const text = readFileSync(join(root, file), "utf8").replace(/\r\n?/g, "\n");
  let stack = textmate.INITIAL;
  text.split("\n").forEach((line, i) => {
    const r = grammar.tokenizeLine(line, stack);
    stack = r.ruleStack;
    const trimmed = line.trimStart();
    if (trimmed === "") return;
    lines += 1;
    const first = r.tokens.find((t) => line.slice(t.startIndex, t.endIndex).trim() !== "");
    const scopes = first ? first.scopes : [];
    const fail = (why) => {
      failures += 1;
      console.error(`${file}:${i + 1}: ${why}\n  ${line}\n  ${scopes.join(" ")}`);
    };
    if (trimmed.startsWith("#")) {
      if (!scopes.includes("comment.line.number-sign.tok")) fail("a comment line is not a comment");
    } else if (trimmed.startsWith("|")) {
      if (!scopes.includes("meta.table.row.tok")) fail("a table row is not a row");
    } else if (keywords.has(trimmed.split(/\s/)[0]) && !keywordScopes.some((s) => scopes.includes(s))) {
      fail("the first word of a statement is not a keyword");
    }
    if (!trimmed.startsWith('"') && scopes.some((s) => s.startsWith("string"))) {
      fail("a string runs over from the line before");
    }
  });
}
if (failures > 0) {
  console.error(`${failures} problem(s)`);
  process.exit(1);
}
console.log(`grammar: ${files.length} documents, ${lines} lines highlighted as expected`);
