/*
 * tok.h: C interface of the tok engine (library tokengine).
 *
 * The engine reads and checks tok documents, prints them in canonical form, exports
 * BPMN 2.0, SVG, DMN and Camunda forms, imports BPMN and DMN files, evaluates decisions
 * and runs simulations. This header declares its C interface.
 *
 * Conventions
 *
 *   - Every string passed in is NUL-terminated UTF-8. The functions do not keep the
 *     pointers after they return.
 *   - Options are a JSON object (for example {"lang": "pl"}). NULL or an empty string
 *     means no options. An unknown option is an error.
 *   - Every function except tok_version and tok_abi_version returns a newly allocated,
 *     NUL-terminated UTF-8 string holding one JSON object:
 *
 *         {"ok": VALUE}                                  on success
 *         {"error": {"kind": KIND, "message": TEXT}}     on failure
 *
 *     The caller frees it with tok_free, exactly once. The functions never return NULL.
 *   - Error kinds: "invalid_argument" (NULL text, invalid UTF-8, malformed or unknown
 *     options), "document" (the document has errors; the error also has "diagnostics",
 *     a list of diagnostics as tok_check returns them), "render" (nothing to draw, or a
 *     layout failure), "import" (a file that cannot be imported), "unknown_decision",
 *     "decision" (a violated hit policy), "feel" (an expression that cannot be
 *     evaluated), "distribution" (an invalid distribution), "simulation" (a model the
 *     simulator cannot run), "io" and "internal" (an error in the engine itself).
 *   - The functions are reentrant and may be called from several threads at once.
 *     No function lets a Rust panic unwind into the caller.
 *
 * The options and results of every function are described in docs/c-api.md.
 */
#ifndef TOK_H
#define TOK_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Version of the C interface implemented by this header. tok_abi_version() returns the
   version of the loaded library; it is incremented only by incompatible changes. */
#define TOK_ABI_VERSION 1

/* Version of the engine, for example "1.0.0". The string is static: do not free it. */
const char *tok_version(void);

/* Version of the C interface of the loaded library (TOK_ABI_VERSION). */
uint32_t tok_abi_version(void);

/* Free a string returned by a function of this library. NULL is ignored. */
void tok_free(char *s);

/* Diagnostics of a document (tok check).
   Options: "lang" ("en" or "pl"), "level" ("basic", "analytic", "full"),
   "bpmn" (boolean, BPMN rules; default true), "class" (a BPMN conformance class).
   Result: {"errors": N, "bpmn_class": CLASS, "diagnostics": [{"code", "severity",
   "line", "id", "message"}, ...]}. */
char *tok_check(const char *text, const char *options);

/* Canonical text of a document (tok fmt). Options: "lang". Result: the text. */
char *tok_fmt(const char *text, const char *options);

/* BPMN 2.0 XML (tok bpmn). Options: "lang", "layout" (boolean, default true),
   "sidecar" (text of a layout sidecar). Result: the XML text. */
char *tok_bpmn(const char *text, const char *options);

/* SVG picture of the diagram (tok svg). Options: "lang", "title", "sidecar".
   Result: the SVG text. */
char *tok_svg(const char *text, const char *options);

/* Every file of tok export. Options: "lang", "name" (stem of the BPMN and SVG files,
   default "model"), "sidecar". Result: an object file name -> content. */
char *tok_export(const char *text, const char *options);

/* A BPMN 2.0 or DMN file as a tok document (tok import). Options: "format" ("auto",
   "bpmn", "dmn"), "flow_ids" (boolean, default true), "layout" (boolean, default true).
   Result: {"format", "text", "warnings": [...], "layout": SIDECAR TEXT or null}. */
char *tok_import(const char *xml, const char *options);

/* The value of a decision (tok eval). Options: "decision" (required), "values" (an
   object input name -> JSON value), "lang". Result: the value as JSON. */
char *tok_eval(const char *text, const char *options);

/* A simulation run (tok sim). Options: "cases", "seed", "lang", "log" (boolean, default
   true), "cases_table" (boolean, default true), "csv" (boolean), "xes" (boolean).
   Result: {"kpi", "report", "log", "cases", "log_csv", "cases_csv", "xes"}; the last
   three only when requested. */
char *tok_sim(const char *text, const char *options);

/* The simulation model of a document: parameters, resources, variables, nodes with
   parsed distributions and timers, sequence flows. Options: "cases", "seed", "lang". */
char *tok_sim_model(const char *text, const char *options);

/* A distribution of the simulation statements ("tri(5m, 10m, 20m)"), parsed:
   {"name", "kind", "args", "labels"}. No options. */
char *tok_sim_dist(const char *text, const char *options);

/* An expression evaluated as the simulator evaluates conditions and scripts.
   Options: "vars" (an object of case variables), "lang07" (boolean: FEEL of language
   0.7). Result: {"value", "truthy"}; an expression that cannot be evaluated is an
   error of kind "feel". */
char *tok_sim_eval(const char *expr, const char *options);

/* The tok command line: args is a JSON array of the arguments after the program name
   (["check", "model.tok", "--lang", "pl"]). The command writes to the standard output
   and error streams of the process, as the tok program does. No options.
   Result: {"status": EXIT STATUS}. */
char *tok_cli(const char *args, const char *options);

#ifdef __cplusplus
}
#endif

#endif /* TOK_H */
