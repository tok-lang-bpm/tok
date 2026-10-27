# FEEL subset used by tok

tok writes conditions (`when=`), script expressions (`expr=`), event conditions (`condition=`), correlation keys (`correlation=`) and decision table cells in **FEEL**, the expression language of DMN (DMN 1.5, chapter 10). Version 0.1 supports a subset of FEEL. It is large enough for the processes of an information institution, and small enough to implement the same way in Python, Julia and R.

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
