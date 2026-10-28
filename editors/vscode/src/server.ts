// Where the language server is: the setting tok.server.path, else the program bundled
// with the extension (server/tok-lsp), else tok-lsp on the PATH.

import * as fs from "fs";
import * as path from "path";

export function resolveServer(extensionPath: string, setting: string): string {
  if (setting.trim() !== "") {
    return setting.trim();
  }
  const exe = process.platform === "win32" ? "tok-lsp.exe" : "tok-lsp";
  const bundled = path.join(extensionPath, "server", exe);
  if (fs.existsSync(bundled)) {
    return bundled;
  }
  return "tok-lsp";
}
