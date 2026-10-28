import glob
import os
import unittest

from tok import bpmnlib
from tok.bpmn_export import export_tree, semantic_xml
from tok.check import check
from tok.parser import parse
from tok.printer import fmt

HERE = os.path.dirname(os.path.abspath(__file__))


def language_version(path):
    """The language version in the header of a tok document (None without a header)."""
    with open(path, encoding="utf-8") as f:
        for line in f.read().lstrip("\ufeff").splitlines():
            words = line.split("#", 1)[0].split()
            if words:
                return words[1] if words[0] == "tok" and len(words) > 1 else None
    return None


# the examples of languages 0.1 and 0.3, which this implementation reads (frozen at 0.3);
# the examples of newer languages are tested by the engine
EXAMPLES = [p for p in sorted(glob.glob(os.path.join(HERE, "..", "..", "examples", "*.tok")))
            if language_version(p) in ("0.1", "0.3", None)]


def codes(text, **kw):
    doc, diags = parse(text)
    if not any(d.severity == "error" for d in diags):
        diags += check(doc, **kw)
    return sorted(d.code for d in diags)


LINEAR = '''tok 0.1 "Linear"
start s "Start"
user a "Sprawdzenie wniosku"
end e "Koniec"
flow s -> a -> e
'''


class LexerParser(unittest.TestCase):
    def test_linear(self):
        doc, diags = parse(LINEAR)
        self.assertEqual(diags, [])
        self.assertEqual(list(doc.nodes), ["s", "a", "e"])
        self.assertEqual([(f.source, f.target) for f in doc.flows], [("s", "a"), ("a", "e")])
        self.assertEqual(doc.nodes["a"].label, "Sprawdzenie wniosku")

    def test_header_required(self):
        self.assertIn("P005", codes("start s\n"))

    def test_tab_indentation(self):
        self.assertIn("T002", codes('tok 0.1\npool p\n\tlane l\n'))

    def test_unknown_keyword(self):
        self.assertIn("P001", codes('tok 0.1\nstep s\n'))

    def test_duplicate_id(self):
        self.assertIn("R001", codes('tok 0.1\nstart s\nend s\n'))

    def test_non_ascii_id(self):
        self.assertIn("R004", codes('tok 0.1\nstart zażółć\n'))

    def test_labelled_chain_rejected(self):
        self.assertIn("R020", codes('tok 0.1\nstart s\ntask a\nend e\nflow s -> a -> e "x"\n'))

    def test_string_escapes(self):
        doc, _ = parse('tok 0.1\nscript a "A" expr="x = \\"y\\""\n')
        self.assertEqual(doc.nodes["a"].attrs["expr"], 'x = "y"')

    def test_lanes_and_pools(self):
        doc, diags = parse('''tok 0.1 "P"
pool lib "Biblioteka"
  lane a "A"
    start s
  lane b "B"
    lane b1 "B1"
      end e
pool ext "Dostawca"
flow s -> e
message e -> ext "Zamówienie"
''')
        self.assertEqual(diags, [])
        self.assertFalse(doc.pools[0].blackbox)
        self.assertTrue(doc.pools[1].blackbox)
        self.assertEqual(doc.nodes["e"].lane, "b1")
        self.assertEqual(doc.lanes["b1"].parent, "b")
        self.assertEqual(doc.nodes["s"].scope, "lib-process")

    def test_decision_table(self):
        doc, diags = parse('''tok 0.1
decision d "D" hit=first
  | wiek: number | typ: string || limit: number || uwagi |
  | < 18         | -           || 5             || młodzi |
  | -            | "student"   || 15            ||       |
''')
        self.assertEqual(diags, [])
        d = doc.decisions["d"]
        self.assertEqual(d.inputs, [("wiek", "number"), ("typ", "string")])
        self.assertEqual(d.rules[0].inputs, ["< 18", "-"])
        self.assertEqual(d.rules[0].annotation, "młodzi")
        self.assertIsNone(d.rules[1].annotation)

    def test_row_cell_count(self):
        self.assertIn("D002", codes('tok 0.1\ndecision d\n  | a: number || b: number |\n  | 1 | 2 || 3 |\n'))


class Formatting(unittest.TestCase):
    def test_examples_idempotent(self):
        for path in EXAMPLES:
            with open(path, encoding="utf-8") as f:
                doc, diags = parse(f.read())
            once = fmt(doc)
            twice = fmt(parse(once)[0])
            self.assertEqual(once, twice, path)

    def test_chain_merging(self):
        doc, _ = parse('tok 0.1\nstart s\ntask a\nend e\nflow s -> a\nflow a -> e\n')
        self.assertIn("flow s -> a -> e", fmt(doc))

    def test_comments_kept(self):
        doc, _ = parse('# about\ntok 0.1\n# the start\nstart s  # first\n')
        out = fmt(doc)
        self.assertIn("# about", out)
        self.assertIn("# the start\nstart s  # first", out)


class Checks(unittest.TestCase):
    def test_examples_have_no_errors(self):
        for path in EXAMPLES:
            with open(path, encoding="utf-8") as f:
                text = f.read()
            errors = [c for c in codes(text) if c[0] in "TPREC" and not c.startswith("C011")]
            self.assertEqual(errors, [], path)

    def test_unknown_reference(self):
        self.assertIn("R002", codes('tok 0.1\nstart s\nflow s -> nowhere\n'))

    def test_cross_scope_flow(self):
        text = 'tok 0.1\nstart s\nsub x\n  start s2\n  end e2\nend e\nflow s -> e2\nflow s2 -> e2\nflow x -> e\n'
        self.assertIn("R021", codes(text))

    def test_message_flow_same_pool(self):
        text = 'tok 0.1\npool p\n  start s\n  end e\nflow s -> e\nmessage s -> e\n'
        self.assertIn("R022", codes(text))

    def test_bpmn_rule_from_validator(self):
        text = 'tok 0.1\nstart s\nxor g "Czy?"\ntask a\nend e\nflow s -> g\nflow g -> a\nflow g -> e\nflow a -> e\n'
        self.assertIn("W021", codes(text))

    def test_executable_profile(self):
        text = 'tok 0.1 executable\nstart s\nscript a\nend e\nflow s -> a -> e\n'
        self.assertIn("C040", codes(text))

    def test_level_basic(self):
        text = 'tok 0.1\nstart s timer=P1D\nend e\nflow s -> e\n'
        self.assertIn("R040", codes(text, level="basic"))
        self.assertNotIn("R040", codes(text, level="analytic"))


class Export(unittest.TestCase):
    def test_examples_valid_bpmn(self):
        for path in EXAMPLES:
            with open(path, encoding="utf-8") as f:
                doc, _ = parse(f.read())
            m = bpmnlib.Model(export_tree(doc))
            errors = [i["kod"] for i in bpmnlib.validate(m) if i["poziom"] == "błąd"]
            self.assertEqual(errors, [], path)
            bpmnlib.layout(m)
            self.assertIn("<svg", bpmnlib.render_svg(m))

    def test_semantic_xml_stable(self):
        doc, _ = parse(LINEAR)
        a = semantic_xml(doc)
        self.assertEqual(a, semantic_xml(parse(fmt(doc))[0]))
        self.assertIn('<bpmn:userTask id="a" name="Sprawdzenie wniosku"', a)

    def test_camunda_extensions(self):
        doc, _ = parse('tok 0.1 executable\nstart s\nuser a form=f\nend e\nflow s -> a -> e\n'
                       'form f\n  field x "X"\n')
        xml = semantic_xml(doc)
        self.assertIn("zeebe:userTask", xml)
        self.assertIn('formId="f"', xml)
        self.assertIn('modeler:executionPlatform="Camunda Cloud"', xml)


if __name__ == "__main__":
    unittest.main()
