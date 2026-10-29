# Security

tok reads text, BPMN, DMN and XES files and writes files; its MCP server and language server read requests from the program that started them, and the playground runs in the browser without a server. A security problem is, for example, a file that makes a tok program crash, hang, use unbounded memory or write outside the folder it was given.

## Supported versions

The latest release of the 1.x series receives fixes.

## Reporting a problem

Report it privately through the **Security** tab of https://github.com/tok-lang-bpm/tok (*Report a vulnerability*), with the file that triggers it and the command or request you used. Please do not open a public issue for it. You will get an answer within a week, and the fix is released as a patch version with a note in the changelog.

The engine is tested with fuzzing ([docs/fuzzing.md](docs/fuzzing.md)); inputs found that way that crash or exhaust the engine are handled as security problems.
