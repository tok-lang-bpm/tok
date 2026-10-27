import os
import unittest
import xml.etree.ElementTree as ET

from tok import dmn
from tok.parser import parse

EXAMPLE = os.path.join(os.path.dirname(__file__), "..", "..", "examples", "propozycja-zakupu.tok")


def decision(text):
    doc, diags = parse('tok 0.1 "t"\n\n' + text)
    assert not diags, [d.message() for d in diags]
    return next(iter(doc.decisions.values()))


class ExampleDecisionTest(unittest.TestCase):
    def setUp(self):
        with open(EXAMPLE, encoding="utf-8") as f:
            self.doc, diags = parse(f.read())
        self.assertEqual(diags, [])
        self.dec = self.doc.decisions["tryb-akceptacji"]

    def test_rules(self):
        ev = dmn.evaluate_decision
        self.assertEqual(ev(self.dec, {"rodzaj": "e-book", "cena": 40}), {"tryb": "kierownik"})
        self.assertEqual(ev(self.dec, {"rodzaj": "drukowana", "cena": 150}), {"tryb": "automatyczny"})
        self.assertEqual(ev(self.dec, {"rodzaj": "drukowana", "cena": 150.01}), {"tryb": "kierownik"})
        self.assertEqual(dmn.single_result(self.dec, ev(self.dec, {"rodzaj": "e-book", "cena": 1})), "kierownik")

    def test_no_diagnostics(self):
        self.assertEqual(dmn.check_decision(self.dec), [])

    def test_export(self):
        xml = dmn.export_dmn(self.dec, self.doc)
        root = ET.fromstring(xml.split("\n", 1)[1])
        ns = {"d": dmn.DMN}
        table = root.find("d:decision/d:decisionTable", ns)
        self.assertEqual(root.find("d:decision", ns).get("id"), "tryb-akceptacji")
        self.assertEqual(table.get("hitPolicy"), "FIRST")
        self.assertEqual(len(table.findall("d:input", ns)), 2)
        self.assertEqual(table.find("d:output", ns).get("name"), "tryb")
        rules = table.findall("d:rule", ns)
        self.assertEqual(len(rules), 3)
        entries = rules[1].findall("d:inputEntry/d:text", ns)
        self.assertEqual([e.text for e in entries], ['"drukowana"', "<= 150"])
        self.assertEqual(rules[0].find("d:description", ns).text, "licencje")


class HitPolicyTest(unittest.TestCase):
    TABLE = """decision limit "Limit" hit=%s
  | wiek: number | konto: string || limit: number |
  | < 18         | -             || 5             |
  | >= 18        | "student"     || 15            |
  | >= 18        | "pracownik"   || 30            |
  | -            | -             || 10            |
"""

    def test_first_and_collect(self):
        first = decision(self.TABLE % "first")
        self.assertEqual(dmn.evaluate_decision(first, {"wiek": 20, "konto": "student"}), {"limit": 15})
        coll = decision(self.TABLE % "collect")
        self.assertEqual(dmn.evaluate_decision(coll, {"wiek": 20, "konto": "student"}),
                         [{"limit": 15}, {"limit": 10}])
        self.assertEqual(dmn.single_result(coll, [{"limit": 15}]), [15])

    def test_unique_violation_and_overlap(self):
        uniq = decision(self.TABLE % "unique")
        with self.assertRaises(dmn.DecisionError):
            dmn.evaluate_decision(uniq, {"wiek": 20, "konto": "student"})
        codes = [(c, a["a"], a["b"]) for c, _, a in dmn.check_decision(uniq)]
        self.assertIn(("D010", 1, 4), codes)
        self.assertIn(("D010", 2, 4), codes)
        self.assertNotIn(("D010", 1, 2), codes)
        self.assertNotIn(("D010", 2, 3), codes)

    def test_no_match(self):
        d = decision("decision d hit=unique\n  | x: number || y: string |\n  | < 0 || \"neg\" |\n")
        self.assertIsNone(dmn.evaluate_decision(d, {"x": 5}))

    def test_cell_diagnostics(self):
        d = decision("decision d hit=sometimes\n  | x: number || y: string |\n  | < > 0 || {a: 1} |\n")
        codes = sorted(c for c, _, _ in dmn.check_decision(d))
        self.assertEqual(codes, ["D003", "F001", "F002"])
        empty = decision("decision e\n  | x: number || y: string |\n")
        self.assertEqual([c for c, _, _ in dmn.check_decision(empty)], ["D005"])


if __name__ == "__main__":
    unittest.main()
