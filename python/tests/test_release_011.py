import io
import os
import re
import sys
import tempfile
import unittest
from contextlib import redirect_stdout

from tok import cli
from tok.bpmn_export import semantic_xml
from tok.check import check
from tok.feel import check_expr, check_unary, detail_pl
from tok.parser import parse


def diags(text):
    doc, d = parse(text)
    return d + check(doc)


class FeelPolishDetails(unittest.TestCase):
    def test_every_detail_is_translated(self):
        samples = ["a > > 1", "a # b", "a then", "a >", '"abc', "a ** 2", "{a:1}", "for x in y return x",
                   "string length()", "a.", "foo(1)", "[1..", "if a then b", ""]
        for s in samples:
            res = check_expr(s)
            self.assertIsNotNone(res, s)
            pl = detail_pl(res[1])
            self.assertNotEqual(pl, res[1], "no Polish text for %r: %s" % (s, res[1]))
            self.assertNotRegex(pl, r"\b(unexpected|expected|found|expressions|function|empty)\b")
        res = check_unary("")
        self.assertEqual(detail_pl(res[1]), "pusta komórka; dla dowolnej wartości wpisz „-”")

    def test_polish_message_in_check(self):
        text = 'tok 0.1\nstart s\nxor g "Czy?"\nend a\nend b\nflow s -> g\nflow g -> a "tak" when="x > > 1"\n' \
               'flow g -> b "nie" default\n'
        d = [x for x in diags(text) if x.code == "F002"]
        self.assertEqual(len(d), 1)
        self.assertEqual(d[0].message("pl"), "błąd składni FEEL: nieoczekiwany element „>”")
        self.assertEqual(d[0].message("en"), "FEEL syntax error: unexpected '>'")

    def test_polish_message_in_decision_cell(self):
        text = 'tok 0.1\ndecision d\n  | a: number || b: number |\n  | < < 1 || 1 |\n'
        d = [x for x in diags(text) if x.code == "F002"]
        self.assertEqual(len(d), 1)
        self.assertIn("(< < 1)", d[0].message("pl"))
        self.assertNotIn("unexpected", d[0].message("pl"))


class OverlapExample(unittest.TestCase):
    def test_concrete_values_for_any_cells(self):
        text = '''tok 0.1
decision d "D"
  | cel: string | wiek: number || tryb: string |
  | -           | > 50         || "a"          |
  | "naukowy"   | -            || "b"          |
  | -           | -            || "c"          |
'''
        warnings = [x for x in diags(text) if x.code == "D010"]
        self.assertEqual(len(warnings), 3)
        for w in warnings:
            self.assertNotIn("=-", w.message("en"))
            self.assertIn('cel="naukowy"', w.message("en"))
            self.assertIn("wiek=51", w.message("en"))

    def test_null_only_without_candidates(self):
        text = 'tok 0.1\ndecision d\n  | s: string || r: string |\n  | - || "a" |\n  | - || "b" |\n'
        w = [x for x in diags(text) if x.code == "D010"]
        self.assertEqual(len(w), 1)
        self.assertIn("s=null", w[0].message("en"))


class ExportPaths(unittest.TestCase):
    def test_native_separators(self):
        here = os.path.dirname(os.path.abspath(__file__))
        src = os.path.join(here, "..", "..", "examples", "propozycja-zakupu.tok")
        with tempfile.TemporaryDirectory() as tmp:
            out = io.StringIO()
            with redirect_stdout(out):
                code = cli.main(["export", src, "-o", tmp + "/out"])
            self.assertEqual(code, 0)
            lines = [x for x in out.getvalue().splitlines() if x.strip()]
            self.assertTrue(lines)
            for line in lines:
                self.assertEqual(line, os.path.normpath(line))
                other = "/" if os.sep == "\\" else "\\"
                self.assertNotIn(other, line)


class BpmnPassGate(unittest.TestCase):
    def test_no_bpmn_notes_next_to_feel_errors(self):
        text = 'tok 0.1\nstart s\ntask t\nxor g "Czy?"\nend a\nend b\nflow s -> t -> g\n' \
               'flow g -> a "tak" when="x > > 1"\nflow g -> b "nie" default\n'
        codes = [x.code for x in diags(text)]
        self.assertIn("F002", codes)
        self.assertFalse([c for c in codes if c[0] in "EWL"], codes)

    def test_bpmn_rules_still_run_on_valid_documents(self):
        text = 'tok 0.1\nstart s\ntask t\nend e\nflow s -> t -> e\n'
        self.assertIn("L030", [x.code for x in diags(text)])


class ExporterVersion(unittest.TestCase):
    def test_language_version_in_xml(self):
        doc, _ = parse('tok 0.1 "T"\nstart s\nend e\nflow s -> e\n')
        self.assertIn('exporterVersion="0.1"', semantic_xml(doc))


if __name__ == "__main__":
    unittest.main()
