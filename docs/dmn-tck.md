# DMN TCK conformance gate

The test cases of the DMN Technology Compatibility Kit (TCK) are the conformance gate of FEEL, decision evaluation and DMN import of language 0.7 (`design-0.7.md` §9). The models and test files of compliance levels 2 and 3 that tok supports are kept in `conformance/dmn-tck`, unmodified copies of the TCK's files; they keep their licence, Apache-2.0, and their source and attribution are given in [`conformance/dmn-tck/README.md`](../conformance/dmn-tck/README.md).

## What the gate checks

For every model the test `rust/tok-core/tests/dmn_tck.rs`:

1. imports it (`tok import`), prints the tok text, parses it again and checks that the text is in canonical form;
2. evaluates, for every test case of the test files, every expected result: the decision named by the result node, with the input nodes as input data (`tok eval`);
3. compares the result with the expected value: numbers by value (a result whose expected value has eight or more decimals, computed by the TCK in floating point, may differ by 10⁻¹⁰ of it), strings, booleans, dates, times and durations by FEEL equality, lists and contexts element by element; an expected error (`errorResult="true"`) is `null`;
4. fails when a model does not import or print, when a result fails that the baseline `rust/tok-core/tests/fixtures/dmn-tck/baseline.tsv` does not list, or when a listed result passes (the baseline stays exact). Every entry of the baseline has a reason.

After an improvement, `TOK_BLESS=1 cargo test -p tok-core --test dmn_tck` stores the new baseline (reasons from `rust/tok-core/tests/fixtures/dmn-tck/reasons.tsv`).

## Selection

All test folders of compliance levels 2 and 3 are included except those that need what tok does not implement:

| Excluded | Reason |
|---|---|
| `0076-feel-external-java` | externally defined functions (Java) |
| `0085-decision-services` | decision services |
| `0086-import`, `0089-nested-inputdata-imports`, `1160-multiple-imports` | models that import other models |

Test cases of the type `decisionService` or `bkm` (which invoke a decision service or a business knowledge model directly instead of a decision) are not evaluated. The model `0088-no-decision-logic` is imported and printed, but all its test cases invoke a decision service, so it has no results in the table below.

## Results

| Level | Models | Models passing completely | Results | Passing |
|---|---|---|---|---|
| 2 | 28 | 28 | 126 | 126 (100.0 %) |
| 3 | 114 | 108 | 3331 | 3313 (99.5 %) |

Results that fail, with the reasons of the baseline:

| Test | Cases | Reason |
|---|---|---|
| `0068-feel-equality` | datetime_008, datetime_008_a, datetime_008_b | zone identifiers whose offset depends on the date (Europe/Paris) need the IANA time zone database, which tok does not include |
| `0068-feel-equality` | range_010, range_011, range_012 | a unary test of one value (=10, !=10) used as a range value is not supported |
| `0074-feel-properties` | range_011 | a unary test of one value (=10) used as a range value is not supported |
| `0082-feel-coercion` | decisionService_002_c | decision services are not supported |
| `0092-feel-lambda` | 013 | decision services are not supported |
| `0100-arithmetic` | subtract_lhs_dateAndTime_minus_rhs_dateAndTime_007, subtract_lhs_dateAndTime_minus_rhs_dateAndTime_008, subtract_lhs_dateAndTime_minus_rhs_dateAndTime_012, subtract_lhs_dateAndTime_minus_rhs_date_002, subtract_lhs_dateAndTime_minus_rhs_date_004, subtract_lhs_date_minus_rhs_dateAndTime_003, subtract_lhs_date_minus_rhs_dateAndTime_005 | zone identifiers whose offset depends on the date (Europe/Paris, Asia/Dhaka) need the IANA time zone database, which tok does not include |
| `1151-boxed-filter` | 004, 005 | a boxed filter is imported as a FEEL filter, which drops elements whose match is not a boolean instead of giving null |

## Coverage per test

| Level | Test | Passing | Results |
|---|---|---|---|
| 2 | `0001-input-data-string` | 1 | 1 |
| 2 | `0002-input-data-number` | 1 | 1 |
| 2 | `0003-input-data-string-allowed-values` | 1 | 1 |
| 2 | `0004-simpletable-U` | 3 | 3 |
| 2 | `0005-simpletable-A` | 3 | 3 |
| 2 | `0006-simpletable-P1` | 3 | 3 |
| 2 | `0007-simpletable-P2` | 3 | 3 |
| 2 | `0008-LX-arithmetic` | 3 | 3 |
| 2 | `0009-invocation-arithmetic` | 3 | 3 |
| 2 | `0010-multi-output-U` | 3 | 3 |
| 2 | `0100-feel-constants` | 2 | 2 |
| 2 | `0101-feel-constants` | 6 | 6 |
| 2 | `0102-feel-constants` | 4 | 4 |
| 2 | `0105-feel-math` | 33 | 33 |
| 2 | `0106-feel-ternary-logic` | 18 | 18 |
| 2 | `0107-feel-ternary-logic-not` | 3 | 3 |
| 2 | `0108-first-hitpolicy` | 3 | 3 |
| 2 | `0109-ruleOrder-hitpolicy` | 3 | 3 |
| 2 | `0110-outputOrder-hitpolicy` | 3 | 3 |
| 2 | `0111-first-hitpolicy-singleoutputcol` | 3 | 3 |
| 2 | `0112-ruleOrder-hitpolicy-singleinoutcol` | 3 | 3 |
| 2 | `0113-outputOrder-hitpolicy-singleinoutcol` | 3 | 3 |
| 2 | `0114-min-collect-hitpolicy` | 3 | 3 |
| 2 | `0115-sum-collect-hitpolicy` | 3 | 3 |
| 2 | `0116-count-collect-hitpolicy` | 3 | 3 |
| 2 | `0117-multi-any-hitpolicy` | 3 | 3 |
| 2 | `0118-multi-priority-hitpolicy` | 3 | 3 |
| 2 | `0119-multi-collect-hitpolicy` | 3 | 3 |
| 3 | `0001-filter` | 1 | 1 |
| 3 | `0002-string-functions` | 4 | 4 |
| 3 | `0003-iteration` | 1 | 1 |
| 3 | `0004-lending` | 11 | 11 |
| 3 | `0005-literal-invocation` | 3 | 3 |
| 3 | `0006-join` | 1 | 1 |
| 3 | `0007-date-time` | 19 | 19 |
| 3 | `0008-listGen` | 10 | 10 |
| 3 | `0009-append-flatten` | 10 | 10 |
| 3 | `0010-concatenate` | 6 | 6 |
| 3 | `0011-insert-remove` | 6 | 6 |
| 3 | `0012-list-functions` | 19 | 19 |
| 3 | `0013-sort` | 3 | 3 |
| 3 | `0014-loan-comparison` | 2 | 2 |
| 3 | `0016-some-every` | 8 | 8 |
| 3 | `0017-tableTests` | 4 | 4 |
| 3 | `0020-vacation-days` | 7 | 7 |
| 3 | `0021-singleton-list` | 5 | 5 |
| 3 | `0030-user-defined-functions` | 2 | 2 |
| 3 | `0031-user-defined-functions` | 3 | 3 |
| 3 | `0032-conditionals` | 6 | 6 |
| 3 | `0033-for-loops` | 4 | 4 |
| 3 | `0034-drg-scopes` | 12 | 12 |
| 3 | `0035-test-structure-output` | 3 | 3 |
| 3 | `0036-dt-variable-input` | 24 | 24 |
| 3 | `0037-dt-on-bkm-implicit-params` | 2 | 2 |
| 3 | `0038-dt-on-bkm-explicit-params` | 2 | 2 |
| 3 | `0039-dt-list-semantics` | 2 | 2 |
| 3 | `0040-singlenestedcontext` | 3 | 3 |
| 3 | `0041-multiple-nestedcontext` | 3 | 3 |
| 3 | `0050-feel-abs-function` | 17 | 17 |
| 3 | `0051-feel-sqrt-function` | 15 | 15 |
| 3 | `0052-feel-exp-function` | 15 | 15 |
| 3 | `0053-feel-log-function` | 15 | 15 |
| 3 | `0054-feel-even-function` | 17 | 17 |
| 3 | `0055-feel-odd-function` | 17 | 17 |
| 3 | `0056-feel-modulo-function` | 28 | 28 |
| 3 | `0057-feel-context` | 11 | 11 |
| 3 | `0058-feel-number-function` | 21 | 21 |
| 3 | `0059-feel-all-function` | 19 | 19 |
| 3 | `0060-feel-any-function` | 17 | 17 |
| 3 | `0061-feel-median-function` | 14 | 14 |
| 3 | `0062-feel-mode-function` | 13 | 13 |
| 3 | `0063-feel-stddev-function` | 12 | 12 |
| 3 | `0064-feel-conjunction` | 19 | 19 |
| 3 | `0065-feel-disjunction` | 19 | 19 |
| 3 | `0066-feel-negation` | 6 | 6 |
| 3 | `0067-feel-split-function` | 9 | 9 |
| 3 | `0068-feel-equality` | 108 | 114 |
| 3 | `0069-feel-list` | 35 | 35 |
| 3 | `0070-feel-instance-of` | 142 | 142 |
| 3 | `0071-feel-between` | 38 | 38 |
| 3 | `0072-feel-in` | 327 | 327 |
| 3 | `0073-feel-comments` | 3 | 3 |
| 3 | `0074-feel-properties` | 52 | 53 |
| 3 | `0075-feel-exponent` | 12 | 12 |
| 3 | `0077-feel-nan` | 1 | 1 |
| 3 | `0078-feel-infinity` | 2 | 2 |
| 3 | `0080-feel-getvalue-function` | 14 | 14 |
| 3 | `0081-feel-getentries-function` | 9 | 9 |
| 3 | `0082-feel-coercion` | 32 | 33 |
| 3 | `0083-feel-unicode` | 14 | 14 |
| 3 | `0084-feel-for-loops` | 24 | 24 |
| 3 | `0087-chapter-11-example` | 9 | 9 |
| 3 | `0090-feel-paths` | 4 | 4 |
| 3 | `0091-local-hrefs` | 1 | 1 |
| 3 | `0092-feel-lambda` | 17 | 18 |
| 3 | `0093-feel-at-literals` | 19 | 19 |
| 3 | `0094-feel-product-function` | 13 | 13 |
| 3 | `0095-feel-day-of-year-function` | 19 | 19 |
| 3 | `0096-feel-day-of-week-function` | 12 | 12 |
| 3 | `0097-feel-month-of-year-function` | 12 | 12 |
| 3 | `0098-feel-week-of-year-function` | 19 | 19 |
| 3 | `0099-arithmetic-negation` | 14 | 14 |
| 3 | `0100-arithmetic` | 1080 | 1087 |
| 3 | `0103-feel-is-function` | 50 | 50 |
| 3 | `1100-feel-decimal-function` | 10 | 10 |
| 3 | `1101-feel-floor-function` | 17 | 17 |
| 3 | `1102-feel-ceiling-function` | 17 | 17 |
| 3 | `1103-feel-substring-function` | 11 | 11 |
| 3 | `1104-feel-string-length-function` | 6 | 6 |
| 3 | `1105-feel-upper-case-function` | 8 | 8 |
| 3 | `1106-feel-lower-case-function` | 9 | 9 |
| 3 | `1107-feel-substring-before-function` | 9 | 9 |
| 3 | `1108-feel-substring-after-function` | 10 | 10 |
| 3 | `1109-feel-replace-function` | 28 | 28 |
| 3 | `1110-feel-contains-function` | 10 | 10 |
| 3 | `1111-feel-matches-function` | 40 | 40 |
| 3 | `1115-feel-date-function` | 52 | 52 |
| 3 | `1116-feel-time-function` | 83 | 83 |
| 3 | `1117-feel-date-and-time-function` | 88 | 88 |
| 3 | `1120-feel-duration-function` | 50 | 50 |
| 3 | `1121-feel-years-and-months-duration-function` | 36 | 36 |
| 3 | `1130-feel-interval` | 14 | 14 |
| 3 | `1131-feel-function-invocation` | 8 | 8 |
| 3 | `1140-feel-string-join-function` | 22 | 22 |
| 3 | `1141-feel-round-up-function` | 16 | 16 |
| 3 | `1142-feel-round-down-function` | 16 | 16 |
| 3 | `1143-feel-round-half-up-function` | 16 | 16 |
| 3 | `1144-feel-round-half-down-function` | 16 | 16 |
| 3 | `1145-feel-context-function` | 18 | 18 |
| 3 | `1146-feel-context-put-function` | 30 | 30 |
| 3 | `1147-feel-context-merge-function` | 14 | 14 |
| 3 | `1148-feel-now-function` | 2 | 2 |
| 3 | `1149-feel-today-function` | 2 | 2 |
| 3 | `1150-boxed-conditional` | 3 | 3 |
| 3 | `1151-boxed-filter` | 2 | 4 |
| 3 | `1152-boxed-for` | 2 | 2 |
| 3 | `1153-boxed-some` | 5 | 5 |
| 3 | `1154-boxed-every` | 5 | 5 |
| 3 | `1155-list-replace-function` | 22 | 22 |
| 3 | `1156-range-function` | 56 | 56 |
| 3 | `1157-implicit-conversions` | 6 | 6 |
| 3 | `1161-boxed-list-expression` | 2 | 2 |
