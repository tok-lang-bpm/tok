// The live diagram: a webview beside the editor with the SVG picture the language server
// draws (request tok/svg, the picture of `tok svg`), redrawn as the document changes.

import * as vscode from "vscode";

type Draw = (doc: vscode.TextDocument) => Promise<{ svg: string } | undefined>;

export class DiagramPreview implements vscode.Disposable {
  private panel: vscode.WebviewPanel | undefined;
  private document: vscode.TextDocument | undefined;
  private timer: NodeJS.Timeout | undefined;
  private readonly subscriptions: vscode.Disposable[] = [];

  constructor(private readonly draw: Draw) {
    this.subscriptions.push(
      vscode.workspace.onDidChangeTextDocument((e) => {
        if (this.panel && e.document === this.document) this.schedule();
      }),
      vscode.window.onDidChangeActiveTextEditor((editor) => {
        if (this.panel && editor && editor.document.languageId === "tok" && editor.document !== this.document) {
          this.document = editor.document;
          this.schedule(0);
        }
      }),
    );
  }

  show(doc: vscode.TextDocument): void {
    this.document = doc;
    if (!this.panel) {
      this.panel = vscode.window.createWebviewPanel("tok.diagram", "tok diagram", vscode.ViewColumn.Beside, {
        enableScripts: false,
        retainContextWhenHidden: true,
      });
      this.panel.onDidDispose(() => {
        this.panel = undefined;
      }, null, this.subscriptions);
    } else {
      this.panel.reveal(vscode.ViewColumn.Beside, true);
    }
    this.schedule(0);
  }

  private schedule(delay = 300): void {
    if (this.timer) clearTimeout(this.timer);
    this.timer = setTimeout(() => this.update(), delay);
  }

  private async update(): Promise<void> {
    const doc = this.document;
    const panel = this.panel;
    if (!doc || !panel) return;
    panel.title = `tok: ${doc.isUntitled ? "diagram" : doc.fileName.split(/[\\/]/).pop()}`;
    const r = await this.quietDraw(doc);
    if (this.panel !== panel || this.document !== doc) return;
    panel.webview.html = page(r.svg, r.message);
  }

  /** The picture, or the message of the server when the document has errors. */
  private async quietDraw(doc: vscode.TextDocument): Promise<{ svg?: string; message?: string }> {
    try {
      const r = await this.draw(doc);
      return r ? { svg: r.svg } : { message: "The language server is not running." };
    } catch (e) {
      return { message: e instanceof Error ? e.message : String(e) };
    }
  }

  dispose(): void {
    if (this.timer) clearTimeout(this.timer);
    this.panel?.dispose();
    for (const s of this.subscriptions) s.dispose();
  }
}

function escapeHtml(text: string): string {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

/** The page of the webview: the SVG, or the reason there is none. */
export function page(svg?: string, message?: string): string {
  const body = svg
    ? `<div class="diagram">${svg.replace(/^<\?xml[^>]*>\s*/, "")}</div>`
    : `<p class="message">${escapeHtml(message ?? "Nothing to draw.")}</p>`;
  return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; img-src data:;">
<style>
  body { margin: 0; padding: 12px; background: var(--vscode-editor-background); color: var(--vscode-foreground); font-family: var(--vscode-font-family); }
  .diagram { background: #ffffff; border-radius: 4px; padding: 8px; display: inline-block; }
  .diagram svg { max-width: 100%; height: auto; }
  .message { white-space: pre-wrap; color: var(--vscode-errorForeground); }
</style>
</head>
<body>${body}</body>
</html>`;
}
