# FEEL in tok

tok writes conditions (`when=`), script expressions (`expr=`), event conditions (`condition=`), correlation keys (`correlation=`) and decision logic in **FEEL**, the expression language of DMN (DMN 1.5, chapter 10). Documents of languages 0.1–0.6 use a subset of FEEL (§1–§8). It is large enough for the processes of an information institution, and small enough to implement the same way in Python, Julia and R. Documents of languages 0.7 and 1.0 use FEEL at DMN conformance level 3 (§9).

An expression outside the subset is still exported unchanged to BPMN and DMN XML, so a process engine with full FEEL (for example Camunda 8) can run it. `tok check` reports it as `F001` (a warning), and the simulator cannot evaluate it. A malformed expression is `F002` (an error).

In tok, expressions are written **without** the leading `=` that Camunda uses. The exporter adds it.

## 1. Values

| Type | Literals | Notes |
|---|---|---|
| number | `150`, `12.5`, `-3` | decimal arithmetic: `0.1 + 0.2 = 0.3` exactly |
| string | `"e-book"`, `"Zamówienie: "` | escapes `\"`, `\\`, `\n`, `\t` |
| boolean | `true`, `false` | |
| null | `null` | the value of a missing variable |
| list | `[1, 2, 3]`, `["a", "b"]` | |
| date | `date("2027-01-15")` | only through the `date()` function |

## 2. Names

A name is a variable of the process instance or of the decision context: letters, digits and `_`, starting with a letter or `_` (`cena`, `numer_zamowienia`). A dot reads a field of a structured value: `osoba.wiek`. A missing variable or field evaluates to `null`.

Names with spaces, which full FEEL allows, are not part of the subset.

## 3. Operators

From the lowest to the highest priority:

| Operator | Meaning | Example |
|---|---|---|
| `if c then a else b` | choice | `if cena > 150 then "kierownik" else "automatyczny"` |
| `or` | disjunction | `rodzaj = "e-book" or cena > 300` |
| `and` | conjunction | `cena > 150 and rodzaj = "drukowana"` |
| `=` `!=` `<` `<=` `>` `>=` | comparison | `tryb = "kierownik"` |
| `x between a and b` | `a <= x <= b` | `wiek between 18 and 65` |
| `x in …` | membership in a list, an interval or equality | `x in [1..10]`, `x in ["a", "b"]` |
| `+` `-` | addition, subtraction; `+` also joins two strings | `cena + 12.5`, `"Nr " + numer` |
| `*` `/` | multiplication, division | `cena * 1.05` |
| `-x` | negation | `-kwota` |
| `not(x)` | negation of a boolean | `not(zgoda)` |

Intervals after `in`: `[a..b]` closed, `(a..b)` or `]a..b[` open, mixed forms such as `[a..b)` allowed.

## 4. Functions

| Function | Result |
|---|---|
| `string length(s)` | number of characters |
| `upper case(s)`, `lower case(s)` | converted string |
| `contains(s, x)`, `starts with(s, x)`, `ends with(s, x)` | boolean |
| `substring(s, start)`, `substring(s, start, length)` | part of a string; `start` counts from 1, a negative `start` counts from the end |
| `abs(n)`, `floor(n)`, `ceiling(n)` | number |
| `round half up(n, scale)` | `round half up(2.345, 2) = 2.35` |
| `min(…)`, `max(…)`, `sum(…)` | a list or several arguments |
| `count(list)` | number of elements |
| `string(x)` | text of a value: `string(12.50) = "12.5"` |
| `number(s)` | number from text |
| `date(s)` | date from `"YYYY-MM-DD"` |

Any other function call is `F001`.

## 5. null and three-valued logic

- A comparison with `null`, or between values of different types, gives `null`, except `=` and `!=`: `null = null` is `true`, `1 = null` is `false`.
- Arithmetic with `null`, with mismatched types or division by zero gives `null`.
- `and`: `false` if either side is `false`, `true` if both are `true`, otherwise `null`. `or` is symmetric.
- A condition on a sequence flow is taken only when it evaluates to `true`. `null` behaves like `false`.

## 6. Unary tests (decision table input cells)

An input cell of a decision table is a **unary test**: a test applied to the value of the column's input expression.

| Cell | Matches when the input value |
|---|---|
| `-` | always (any value, also `null`) |
| `"e-book"`, `150`, `true` | is equal to the value |
| `< 18`, `<= 150`, `> 0`, `>= 18` | compares as given |
| `[18..65]`, `]18..65]`, `(0..10)`, `[1..5[` | lies in the interval |
| `"student", "doktorant"` | satisfies any test in the list |
| `not("student", "doktorant")` | satisfies none of the tests |
| `< limit` | endpoints may be expressions over variables |

Output cells are ordinary expressions (`"kierownik"`, `cena * 0.1`). An output cell `-` means `null`.

## 7. Examples

```
flow potrzebna -> akceptacja "tak" when="tryb = \"kierownik\""
flow zgoda -> scalenie "tak" when="zgoda = true"
script koszt "Wyliczenie kosztu zakupu" expr="cena + 12.5" result=koszt
script termin "Termin odpowiedzi" expr="if pilne then 3 else 14" result=dni
```

```
decision limit "Limit wypożyczeń" hit=first
  | wiek: number | konto: string           || limit: number |
  | < 18         | -                       || 5             |
  | >= 18        | "student", "doktorant"  || 15            |
  | >= 18        | "pracownik"             || 30            |
  | -            | -                       || 10            |
```

## 8. Differences from full FEEL

The subset leaves out:

- contexts (`{a: 1}`), filters (`list[item > 1]`), path expressions on lists;
- `for`, `some`, `every`, `instance of`, function definitions;
- names containing spaces and the `?` symbol in unary tests;
- temporal literals (`@"2027-01-15"`), durations, times and time zones, and all temporal arithmetic;
- exponentiation (`**`);
- built-in functions other than those in section 4.

Decision tables with hit policy `priority` have no output value lists in tok 0.1; the evaluator treats `priority` as `first`. Overlaps under `unique` are detected (`D010`) for cells built from string, boolean and number literals, comparisons and intervals. Cells with variables or `not(…)` are not analysed.

## 9. FEEL of languages 0.7 and 1.0

In a document with the header `tok 0.7` or `tok 1.0` every FEEL text is full FEEL of DMN 1.5 at conformance level 3, evaluated by the Rust engine. Its conformance is measured with the DMN Technology Compatibility Kit (`dmn-tck.md`).

### 9.1 Values and literals

| Type | Literals and constructors |
|---|---|
| number | `150`, `-12.5`, `.5`, `1.23e4`; `number("1 000,5", " ", ",")` |
| string | `"text"` with the escapes `\"`, `\\`, `\n`, `\r`, `\t`, `\uXXXX` |
| boolean, null | `true`, `false`, `null` |
| date | `date("2027-01-15")`, `date(2027, 1, 15)`, `@"2027-01-15"` |
| time | `time("10:30:00")`, `time("10:30:00+01:00")`, `time("10:30:00@Europe/Warsaw")`, `time(10, 30, 0, duration("PT1H"))`, `@"10:30:00"` |
| date and time | `date and time("2027-01-15T10:30:00Z")`, `date and time(d, t)`, `@"2027-01-15T10:30:00"` |
| days and time duration | `duration("P2DT3H")`, `@"PT90M"` |
| years and months duration | `duration("P1Y6M")`, `years and months duration(from, to)` |
| list | `[1, 2, 3]` |
| context | `{nazwa: "Kowalski", "rok urodzenia": 1990}`; entries see the entries before them |
| range | `[1..10]`, `(1..10)`, `]1..10[`, `[1..10)`; in tests also `< 10`, `>= 18` |
| function | `function(x: number) x * 2`; built-in functions by name |

Numbers are decimal with 34 significant digits and rounding half even (`1/3` is `0.3333333333333333333333333333333333`). Earlier documents keep the 28 digits of their subset. Times and dates and times have nanosecond resolution and compare equal at the resolution of a second. Zone identifiers other than `UTC`, `Z` and `Etc/UTC` are kept and compared by name: tok has no time zone database, so arithmetic between a zone identifier and an offset is `null`.

### 9.2 Expressions

| Expression | Meaning |
|---|---|
| `if c then a else b` | `a` when `c` is `true`, otherwise `b` |
| `for x in l, y in 1..3 return e` | a list; an iteration context is a list, `a..b` of integers or dates, or a range, and may use the variables before it; `partial` holds the results so far |
| `some x in l satisfies c`, `every x in l satisfies c` | `null` when `c` is not a boolean for an element |
| `a or b`, `a and b` | three-valued logic |
| `=`, `!=`, `<`, `<=`, `>`, `>=` | values of the same type; `=` of values of different types is `null` |
| `x between a and b`, `x in (1, 2, > 5)`, `x in [1..10]` | unary tests applied to `x` |
| `x instance of list<number>` | types: `number`, `string`, `boolean`, `date`, `time`, `date and time`, `days and time duration`, `years and months duration`, `list<T>`, `context<a: T>`, `range<T>`, `function<T> -> T`, `Any`, `Null`, DMN types of the document |
| `+`, `-`, `*`, `/`, `**`, `-x` | numbers; `+` joins strings; dates, times and durations as DMN 1.5 Table 57 defines (a date counts as the date and time at midnight UTC) |
| `l[item > 1]`, `l[1]`, `l[-1]` | filter (the entries of a context element are names in the filter) and index |
| `a.b`, `l.b` | the entry of a context, the same entry of every element of a list, the properties of dates, times, durations and ranges (`year`, `month`, `day`, `weekday`, `hour`, `minute`, `second`, `time offset`, `timezone`, `days`, `hours`, `minutes`, `seconds`, `years`, `months`, `start`, `end`, `start included`, `end included`) |
| `f(1, 2)`, `f(a: 1, b: 2)` | invocation with positional or named arguments; a list of one value is its element where a parameter needs a single value, and a single value a list of one where it needs a list |
| `// comment`, `/* comment */` | comments |

**Names** may contain spaces and the symbols `. / - ' + *`: `Zaległe opłaty`, `dni-zwloki`. A name is the longest sequence of words and symbols that is a name of the document (an element of the decision requirements graph, an entry, a parameter, a component of a DMN type, a column) or of a built-in function; otherwise it is a single word. A name with dots that is not a variable reads the fields of the variable it starts with.

### 9.3 Built-in functions

| Group | Functions |
|---|---|
| conversion | `date`, `date and time`, `time`, `number`, `string`, `duration`, `years and months duration` |
| boolean | `not` |
| string | `substring`, `string length`, `upper case`, `lower case`, `substring before`, `substring after`, `replace`, `contains`, `starts with`, `ends with`, `matches`, `split`, `string join` |
| list | `list contains`, `count`, `min`, `max`, `sum`, `mean`, `all`, `any`, `sublist`, `append`, `concatenate`, `insert before`, `remove`, `reverse`, `index of`, `union`, `distinct values`, `flatten`, `product`, `median`, `stddev`, `mode`, `sort`, `list replace` |
| number | `decimal`, `floor`, `ceiling`, `round up`, `round down`, `round half up`, `round half down`, `abs`, `modulo`, `sqrt`, `log`, `exp`, `odd`, `even` |
| date and time | `is`, `day of year`, `day of week`, `month of year`, `week of year`, `last day of month`, `now`, `today` |
| range | `before`, `after`, `meets`, `met by`, `overlaps`, `overlaps before`, `overlaps after`, `finishes`, `finished by`, `includes`, `during`, `starts`, `started by`, `coincides`, `range` |
| context | `get value`, `get entries`, `context`, `context put`, `context merge` |

`matches`, `replace` and `split` use the regular expressions of XML Schema and XPath with the flags `s`, `m`, `i`, `x` and `q`. `min`, `max`, `sum`, `mean`, `all`, `any`, `product`, `median`, `stddev` and `mode` take a list or several arguments.

### 9.4 Errors

An expression that is not defined for its values gives `null`: `1 + "a"`, `10 / 0`, `date("2027-02-30")`, a function called with the wrong number or types of arguments. `tok check` reports syntax errors as `F002`, and externally defined functions (`function(x) external …`), which tok does not evaluate, as `F001`.

### 9.5 FEEL in Camunda 8

Camunda 8 evaluates the decisions of an executable document with its own FEEL engine (feel-scala). The conformance runner `conformance/camunda/run.py` deploys the DMN files of conformance case 61 to Camunda 8.10 and compares its results with `tok eval`; they agree. Differences that remain:

- **Names.** Camunda's FEEL does not read names with spaces or with the symbols `-`, `+`, `/`, `*`, `'` and `.` inside a name; it reads such a name written in backticks (`` `Zaległe opłaty` ``), which is not standard FEEL. tok therefore writes the DMN files of a document for Camunda (with the flag `executable` on the header or a pool and `target=camunda`, the default) with every name its parser resolves that is not a plain name — a letter or `_`, then letters, digits and `_`, non-ASCII letters included — in backticks, and such a key of a context literal as a string (`{"do zapłaty": 1}`). Strings, built-in functions and everything else stay as written; documents that are not for Camunda are exported in standard FEEL.
- **Numbers as text.** DMN 1.5 gives FEEL numbers no scale — equal numbers are the same value — and its examples convert numbers to their plain decimal form (`string(1.1) = "1.1"`); the TCK converts only integral numbers to text (`string(decimal(3, 0))` is `"3"`). tok writes a number without trailing zeros: `string(4 * 0.5)` is `"2"`. Camunda keeps the scale of the Java decimal it computed: `"2.0"`. Numbers themselves compare equal in both.
- **Time zones.** Camunda knows the zone identifiers of the IANA database, tok only offsets, `Z`, `UTC` and `Etc/UTC` (§9.1).
