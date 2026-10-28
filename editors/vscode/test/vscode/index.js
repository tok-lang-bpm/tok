// Tests inside VS Code (extension host), run by test/vscode-run.mjs: the extension starts
// tok-lsp, the editor shows its diagnostics, formats through it, and the commands draw
// the diagram and write the BPMN file.

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const vscode = require("vscode");

async function until(test, what, ms = 20000) {
  const deadline = Date.now() + ms;
  for (;;) {
    const value = await test();
    if (value) return value;
    if (Date.now() > deadline) throw new Error(`timed out waiting for ${what}`);
    await new Promise((r) => setTimeout(r, 200));
  }
}

async function run() {
  const folder = fs.mkdtempSync(path.join(os.tmpdir(), "tok-vscode-"));
  const file = path.join(folder, "zamowienie.tok");
  fs.writeFileSync(file, 'tok 0.1   "Zamówienie"\nstart s "Początek"\nuser a "Złożenie"\nend e\nflow s -> a -> x\n');
  const doc = await vscode.workspace.openTextDocument(file);
  assert.strictEqual(doc.languageId, "tok");
  const editor = await vscode.window.showTextDocument(doc);

  const diagnostics = await until(() => {
    const d = vscode.languages.getDiagnostics(doc.uri);
    return d.length > 0 ? d : null;
  }, "diagnostics");
  assert.strictEqual(diagnostics[0].code, "R002");
  assert.strictEqual(diagnostics[0].range.start.line, 4);

  await editor.edit((e) => e.replace(new vscode.Range(4, 15, 4, 16), "e"));
  await until(() => vscode.languages.getDiagnostics(doc.uri).length === 0, "no diagnostics");

  // VS Code turns the edit of the server into minimal edits; the result is the canonical form
  await vscode.commands.executeCommand("editor.action.formatDocument");
  await until(() => doc.getText().startsWith('tok 0.1 "Zamówienie"\n\nstart s'), "the canonical form");

  const symbols = await vscode.commands.executeCommand("vscode.executeDocumentSymbolProvider", doc.uri);
  assert.deepStrictEqual(symbols.map((s) => s.name), ["s", "a", "e"]);

  const defs = await vscode.commands.executeCommand("vscode.executeDefinitionProvider", doc.uri, new vscode.Position(6, 10));
  assert.strictEqual(defs[0].range.start.line, 3);

  await vscode.commands.executeCommand("tok.showDiagram");
  await until(() => vscode.window.tabGroups.all.some((g) => g.tabs.some((t) => t.label.startsWith("tok"))), "the diagram panel");

  await vscode.window.showTextDocument(doc);
  await vscode.commands.executeCommand("tok.exportBpmn");
  const bpmn = path.join(folder, "zamowienie.bpmn");
  await until(() => fs.existsSync(bpmn), "the BPMN file");
  assert.ok(fs.readFileSync(bpmn, "utf8").includes("bpmndi:BPMNDiagram"));
}

module.exports = { run };
