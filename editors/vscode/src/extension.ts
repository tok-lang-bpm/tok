// The tok extension: the language client of tok-lsp, the live diagram preview and the
// commands that export and check a document. The language server does the work; the
// extension asks it with the requests tok/svg, tok/bpmn, tok/export and tok/check
// (docs/lsp.md).

import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import { LanguageClient, LanguageClientOptions, ServerOptions } from "vscode-languageclient/node";

import { DiagramPreview } from "./preview";
import { resolveServer } from "./server";

let client: LanguageClient | undefined;

/** Settings sent to the server at start (initializationOptions). */
function serverSettings(): Record<string, string | null> {
  const config = vscode.workspace.getConfiguration("tok");
  const lang = config.get<string>("lang", "auto");
  const cls = config.get<string>("class", "none");
  return {
    lang: lang === "auto" ? (vscode.env.language.startsWith("pl") ? "pl" : "en") : lang,
    level: config.get<string>("level", "full"),
    class: cls === "none" ? null : cls,
  };
}

async function startClient(context: vscode.ExtensionContext): Promise<void> {
  const command = resolveServer(context.extensionPath, vscode.workspace.getConfiguration("tok").get<string>("server.path", ""));
  const serverOptions: ServerOptions = { command, args: ["--stdio"] };
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ language: "tok" }],
    initializationOptions: serverSettings(),
    outputChannelName: "tok",
  };
  client = new LanguageClient("tok", "tok", serverOptions, clientOptions);
  try {
    await client.start();
  } catch (e) {
    client = undefined;
    const message = e instanceof Error ? e.message : String(e);
    const choice = await vscode.window.showErrorMessage(
      `tok: the language server ${command} could not be started (${message}). Set tok.server.path to the tok-lsp program.`,
      "Open settings",
    );
    if (choice) vscode.commands.executeCommand("workbench.action.openSettings", "tok.server.path");
  }
}

/** The tok document of the active editor, or a message. */
function activeTokDocument(): vscode.TextDocument | undefined {
  const doc = vscode.window.activeTextEditor?.document;
  if (!doc || doc.languageId !== "tok") {
    vscode.window.showInformationMessage("tok: open a .tok document first.");
    return undefined;
  }
  return doc;
}

/** A request to the server about a document; `undefined` when the server is not running. */
async function request<T>(method: string, doc: vscode.TextDocument, extra: object = {}): Promise<T | undefined> {
  if (!client) return undefined;
  return client.sendRequest<T>(method, { textDocument: { uri: doc.uri.toString() }, ...extra });
}

/** A request whose failures are shown to the user. */
async function ask<T>(method: string, doc: vscode.TextDocument, extra: object = {}): Promise<T | undefined> {
  if (!client) {
    vscode.window.showErrorMessage("tok: the language server is not running.");
    return undefined;
  }
  try {
    return await request<T>(method, doc, extra);
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    vscode.window.showErrorMessage(message.startsWith("tok") ? message : `tok: ${message}`);
    return undefined;
  }
}

function stemOf(doc: vscode.TextDocument): string {
  return doc.isUntitled ? "model" : path.basename(doc.fileName, path.extname(doc.fileName));
}

async function exportBpmn(): Promise<void> {
  const doc = activeTokDocument();
  if (!doc) return;
  const r = await ask<{ bpmn: string }>("tok/bpmn", doc);
  if (!r) return;
  let target: vscode.Uri | undefined;
  if (doc.isUntitled) {
    target = await vscode.window.showSaveDialog({ filters: { BPMN: ["bpmn"] }, defaultUri: vscode.Uri.file("model.bpmn") });
  } else {
    target = vscode.Uri.file(path.join(path.dirname(doc.fileName), `${stemOf(doc)}.bpmn`));
  }
  if (!target) return;
  await vscode.workspace.fs.writeFile(target, Buffer.from(r.bpmn, "utf8"));
  const written = target;
  // the command ends when the file is written, not when the message is closed
  vscode.window.showInformationMessage(`tok: BPMN written to ${written.fsPath}`, "Open").then((open) => {
    if (open) vscode.window.showTextDocument(written);
  });
}

async function exportAll(): Promise<void> {
  const doc = activeTokDocument();
  if (!doc) return;
  const r = await ask<{ files: Record<string, string> }>("tok/export", doc, { name: stemOf(doc) });
  if (!r) return;
  const folders = await vscode.window.showOpenDialog({
    canSelectFiles: false,
    canSelectFolders: true,
    defaultUri: doc.isUntitled ? undefined : vscode.Uri.file(path.dirname(doc.fileName)),
    openLabel: "Export here",
  });
  if (!folders || folders.length === 0) return;
  for (const [name, content] of Object.entries(r.files)) {
    await vscode.workspace.fs.writeFile(vscode.Uri.joinPath(folders[0], name), Buffer.from(content, "utf8"));
  }
  vscode.window.showInformationMessage(`tok: ${Object.keys(r.files).length} file(s) written to ${folders[0].fsPath}`);
}

async function check(): Promise<void> {
  const doc = activeTokDocument();
  if (!doc) return;
  const r = await ask<{ errors: number; warnings: number; notes: number; bpmn_class: string }>("tok/check", doc);
  if (!r) return;
  const summary = `tok: ${r.errors} error(s), ${r.warnings} warning(s), ${r.notes} note(s)`;
  if (r.errors + r.warnings + r.notes > 0) {
    vscode.commands.executeCommand("workbench.actions.view.problems");
  }
  if (r.errors > 0) {
    vscode.window.showWarningMessage(summary);
  } else {
    vscode.window.showInformationMessage(`${summary}; BPMN conformance class: ${r.bpmn_class}`);
  }
}

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  await startClient(context);
  // the preview shows the errors of the document itself, without notifications
  const preview = new DiagramPreview((doc) => request<{ svg: string }>("tok/svg", doc));
  context.subscriptions.push(
    vscode.commands.registerCommand("tok.showDiagram", () => {
      const doc = activeTokDocument();
      if (doc) preview.show(doc);
    }),
    vscode.commands.registerCommand("tok.exportBpmn", exportBpmn),
    vscode.commands.registerCommand("tok.exportAll", exportAll),
    vscode.commands.registerCommand("tok.check", check),
    vscode.commands.registerCommand("tok.restartServer", async () => {
      await client?.stop();
      await startClient(context);
    }),
    vscode.workspace.onDidChangeConfiguration(async (e) => {
      if (e.affectsConfiguration("tok.server.path")) {
        await client?.stop();
        await startClient(context);
      } else if (e.affectsConfiguration("tok") && client) {
        client.sendNotification("workspace/didChangeConfiguration", { settings: { tok: serverSettings() } });
      }
    }),
    preview,
  );
  // the bundled server must be executable after the .vsix is unpacked on Linux and macOS
  const bundled = path.join(context.extensionPath, "server", "tok-lsp");
  if (process.platform !== "win32" && fs.existsSync(bundled)) {
    try {
      fs.chmodSync(bundled, 0o755);
    } catch {
      // a read-only installation keeps the mode it has
    }
  }
}

export async function deactivate(): Promise<void> {
  await client?.stop();
}
