# DMN TCK test cases

This folder holds DMN models and their test files from the DMN Technology
Compatibility Kit (DMN TCK), a community project that defines test cases with
expected results for implementations of the Decision Model and Notation (DMN)
standard of the Object Management Group (OMG). They are the conformance gate of
FEEL, decision evaluation and DMN import of language 0.7
(`rust/tok-core/tests/dmn_tck.rs`, results in `docs/dmn-tck.md`).

## Source

- Repository: https://github.com/dmn-tck/tck
- Folders: `TestCases/compliance-level-2/` and `TestCases/compliance-level-3/`
- Commit: `1e349176e5ae689d924a2a2c7a8df25c220d8b07` (branch `master`, 2026-10-19).
- Files taken: for every test folder its DMN model (`*.dmn`) and its test files
  (`*-test-*.xml`). PDF, Word and image files of the folders were left out, as were
  the folders `0076-feel-external-java` (Java functions), `0085-decision-services`
  (decision services) and `0086-import`, `0089-nested-inputdata-imports`,
  `1160-multiple-imports` (models that import other models), which tok does not
  support.
- The files are unmodified copies with their original names, in folders named as in
  the TCK.

## Licence and attribution

The DMN TCK, test cases included, is licensed under the Apache License, Version 2.0;
a copy of the licence is in [`LICENSE-ASL-2.0.txt`](LICENSE-ASL-2.0.txt). The test
cases are the work of the DMN TCK working group and its contributors
(https://github.com/dmn-tck/tck/graphs/contributors).

These files keep their Apache-2.0 licence. They are not covered by the GPL-3.0
licence of the rest of this repository; they are distributed alongside it as
separate, unmodified data files.

Attribution:

> DMN Technology Compatibility Kit, test cases of compliance levels 2 and 3, by the
> DMN TCK working group, https://github.com/dmn-tck/tck (commit 1e34917), licensed
> under the Apache License, Version 2.0
> (https://www.apache.org/licenses/LICENSE-2.0). No changes were made to the files;
> a selection of the folders is included.
