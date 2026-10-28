//! The reference documents of the language: the cheat sheet, the specification, the
//! diagnostic codes, FEEL and the command line, as `docs/` of the repository holds them. The
//! crate keeps copies in `reference/` so that it builds from its own package; the test suite
//! checks that they equal the documents in `docs/`.

/// `docs/cheatsheet.md`: the language on one page (`tok cheatsheet`).
pub const CHEATSHEET: &str = include_str!("../reference/cheatsheet.md");
/// `docs/spec.md`: the language specification (normative).
pub const SPEC: &str = include_str!("../reference/spec.md");
/// `docs/diagnostics.md`: the diagnostic codes in English and Polish.
pub const DIAGNOSTICS: &str = include_str!("../reference/diagnostics.md");
/// `docs/feel.md`: FEEL in tok.
pub const FEEL: &str = include_str!("../reference/feel.md");
/// `docs/cli.md`: the command line.
pub const CLI: &str = include_str!("../reference/cli.md");
