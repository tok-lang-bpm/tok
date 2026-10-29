// Smoke test of the packaged extension, without a display:
//
// 1. the .vsix is unpacked and its manifest, compiled code, grammar and bundled server are
//    checked;
// 2. the bundled tok-lsp is started from the unpacked extension and driven through the
//    protocol: initialize, a document with an error and its diagnostics, the diagram of a
//    correct document (tok/svg, as the preview asks for it), shutdown and exit;
// 3. when the VS Code command line is available (`code` on the PATH, or the variable
//    VSCODE_CLI), VS Code installs the .vsix into an empty extensions folder and lists it.
//
//   node test/smoke.mjs [FILE.vsix]

import { execFileSync, execSync, spawn } from "node:child_process";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { inflateRawSync } from "node:zlib";

const here = dirname(dirname(fileURLToPath(import.meta.url)));
const manifest = JSON.parse(readFileSync(join(here, "package.json"), "utf8"));

function fail(message) {
  console.error(`smoke: ${message}`);
  process.exit(1);
}

// ------------------------------------------------------------------ unpack

/** The entries of a ZIP archive: name → content. */
function unzip(buffer) {
  let eocd = buffer.length - 22;
  while (eocd >= 0 && buffer.readUInt32LE(eocd) !== 0x06054b50) eocd -= 1;
  if (eocd < 0) fail("the .vsix is not a ZIP archive");
  const count = buffer.readUInt16LE(eocd + 10);
  let p = buffer.readUInt32LE(eocd + 16);
  const entries = new Map();
  for (let i = 0; i < count; i++) {
    if (buffer.readUInt32LE(p) !== 0x02014b50) fail("damaged central directory");
    const method = buffer.readUInt16LE(p + 10);
    const size = buffer.readUInt32LE(p + 20);
    const nameLength = buffer.readUInt16LE(p + 28);
    const extraLength = buffer.readUInt16LE(p + 30);
    const commentLength = buffer.readUInt16LE(p + 32);
    const local = buffer.readUInt32LE(p + 42);
    const name = buffer.toString("utf8", p + 46, p + 46 + nameLength);
    const data = local + 30 + buffer.readUInt16LE(local + 26) + buffer.readUInt16LE(local + 28);
    const raw = buffer.subarray(data, data + size);
    entries.set(name, method === 8 ? inflateRawSync(raw) : Buffer.from(raw));
    p += 46 + nameLength + extraLength + commentLength;
  }
  return entries;
}

const vsix =
  process.argv[2] ??
  readdirSync(here)
    .filter((f) => f.endsWith(".vsix"))
    .map((f) => join(here, f))
    .sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs)[0];
if (!vsix || !existsSync(vsix)) fail("no .vsix found; run npm run package first");

const work = mkdtempSync(join(tmpdir(), "tok-vsix-"));
const entries = unzip(readFileSync(vsix));
for (const [name, content] of entries) {
  if (name.endsWith("/")) continue;
  const target = join(work, ...name.split("/"));
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, content);
}
const ext = join(work, "extension");
const packaged = JSON.parse(readFileSync(join(ext, "package.json"), "utf8"));
if (packaged.name !== "tok" || packaged.version !== manifest.version) fail("wrong manifest in the package");
if (packaged.contributes.languages[0].id !== "tok") fail("the language tok is not contributed");
for (const command of ["tok.showDiagram", "tok.exportBpmn", "tok.exportAll", "tok.check"]) {
  if (!packaged.contributes.commands.some((c) => c.command === command)) fail(`command ${command} missing`);
}
const exe = process.platform === "win32" ? "tok-lsp.exe" : "tok-lsp";
for (const file of [
  "out/extension.js",
  "syntaxes/tok.tmLanguage.json",
  "language-configuration.json",
  `server/${exe}`,
]) {
  if (!existsSync(join(ext, ...file.split("/")))) fail(`${file} is not in the package`);
}
// the code is one bundled file: the language client is inside it, and no node_modules
// are packaged
if ([...entries.keys()].some((name) => name.includes("/node_modules/"))) fail("the package holds node_modules");
const scripts = [...entries.keys()].filter((name) => name.endsWith(".js"));
if (scripts.length !== 1) fail(`the package holds ${scripts.length} JavaScript files, not one bundle`);
const bundle = readFileSync(join(ext, "out", "extension.js"), "utf8");
if (!bundle.includes("LanguageClient") || /require\(["']vscode-languageclient/.test(bundle)) {
  fail("out/extension.js does not bundle the language client");
}
console.log(`smoke: ${vsix} holds ${entries.size} entries with the manifest, code, grammar and server`);

// ------------------------------------------------------------------ the server

function lsp(command) {
  const child = spawn(command, ["--stdio"], { stdio: ["pipe", "pipe", "inherit"] });
  let buffer = Buffer.alloc(0);
  const waiting = [];
  const messages = [];
  child.stdout.on("data", (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);
    for (;;) {
      const end = buffer.indexOf("\r\n\r\n");
      if (end < 0) return;
      const length = Number(/Content-Length: (\d+)/i.exec(buffer.toString("ascii", 0, end))[1]);
      if (buffer.length < end + 4 + length) return;
      const message = JSON.parse(buffer.toString("utf8", end + 4, end + 4 + length));
      buffer = buffer.subarray(end + 4 + length);
      messages.push(message);
      for (const w of waiting.splice(0)) w();
    }
  });
  const send = (message) => {
    const body = Buffer.from(JSON.stringify({ jsonrpc: "2.0", ...message }), "utf8");
    child.stdin.write(`Content-Length: ${body.length}\r\n\r\n`);
    child.stdin.write(body);
  };
  const next = async (test) => {
    const deadline = Date.now() + 20000;
    for (;;) {
      const i = messages.findIndex(test);
      if (i >= 0) return messages.splice(i, 1)[0];
      if (Date.now() > deadline) fail("no answer from the server");
      await new Promise((resolve) => {
        waiting.push(resolve);
        setTimeout(resolve, 200);
      });
    }
  };
  const exited = new Promise((resolve) => child.on("exit", (code) => resolve(code)));
  return { send, next, exited };
}

// the extension makes the bundled server executable when it is activated (a .vsix keeps no
// file modes); the unpacked copy here gets the same
if (process.platform !== "win32") chmodSync(join(ext, "server", exe), 0o755);
const server = lsp(join(ext, "server", exe));
const uri = "file:///smoke/zamowienie.tok";
server.send({ id: 1, method: "initialize", params: { processId: null, capabilities: {}, locale: "pl" } });
const init = await server.next((m) => m.id === 1);
if (init.result.serverInfo.name !== "tok-lsp") fail("unexpected initialize result");
server.send({ method: "initialized", params: {} });
const broken = 'tok 0.1 "Zamówienie"\nstart s "Początek"\nend e\nflow s -> x\n';
server.send({ method: "textDocument/didOpen", params: { textDocument: { uri, languageId: "tok", version: 1, text: broken } } });
const diags = await server.next((m) => m.method === "textDocument/publishDiagnostics");
const d = diags.params.diagnostics;
if (d.length !== 1 || d[0].code !== "R002" || !d[0].message.startsWith("nieznany")) {
  fail(`unexpected diagnostics ${JSON.stringify(d)}`);
}
const fixed = broken.replace("-> x", "-> e");
server.send({ method: "textDocument/didChange", params: { textDocument: { uri, version: 2 }, contentChanges: [{ text: fixed }] } });
const clean = await server.next((m) => m.method === "textDocument/publishDiagnostics");
if (clean.params.diagnostics.length !== 0) fail("diagnostics remain after the fix");
server.send({ id: 2, method: "tok/svg", params: { textDocument: { uri } } });
const svg = await server.next((m) => m.id === 2);
if (!svg.result || !svg.result.svg.includes("<svg")) fail("no picture from tok/svg");
server.send({ id: 3, method: "shutdown" });
await server.next((m) => m.id === 3);
server.send({ method: "exit" });
const code = await server.exited;
if (code !== 0) fail(`the server exited with ${code}`);
console.log("smoke: the bundled tok-lsp answered initialize, diagnostics, tok/svg, shutdown and exit");

// ------------------------------------------------------------------ VS Code

function findCode() {
  if (process.env.VSCODE_CLI) return process.env.VSCODE_CLI;
  const names = process.platform === "win32" ? ["code.cmd", "code"] : ["code"];
  for (const dir of (process.env.PATH ?? "").split(process.platform === "win32" ? ";" : ":")) {
    for (const n of names) if (dir && existsSync(join(dir, n))) return join(dir, n);
  }
  return null;
}

const code_ = findCode();
if (code_) {
  const extensions = join(work, "vscode-extensions");
  const user = join(work, "vscode-user");
  const all = (args) => [...args, "--extensions-dir", extensions, "--user-data-dir", user];
  // code.cmd on Windows runs through the shell: one command line with quoted parts
  const run = (args) =>
    process.platform === "win32"
      ? execSync([code_, ...all(args)].map((a) => `"${a}"`).join(" "), { encoding: "utf8" })
      : execFileSync(code_, all(args), { encoding: "utf8" });
  run(["--install-extension", vsix, "--force"]);
  const listed = run(["--list-extensions", "--show-versions"]);
  const id = `${manifest.publisher}.${manifest.name}@${manifest.version}`.toLowerCase();
  if (!listed.toLowerCase().includes(id)) fail(`VS Code does not list ${id}: ${listed}`);
  console.log(`smoke: VS Code installed and lists ${id}`);
} else {
  console.log("smoke: VS Code command line not found; installation into VS Code not tested");
}

rmSync(work, { recursive: true, force: true });
console.log("smoke: passed");
