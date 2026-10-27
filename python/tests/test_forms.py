import json
import os
import unittest

from tok import forms
from tok.parser import parse

EXAMPLE = os.path.join(os.path.dirname(__file__), "..", "..", "examples", "propozycja-zakupu.tok")


class FormExportTest(unittest.TestCase):
    def setUp(self):
        with open(EXAMPLE, encoding="utf-8") as f:
            self.doc, diags = parse(f.read())
        self.assertEqual(diags, [])

    def test_akceptacja(self):
        data = json.loads(forms.export_form(self.doc.forms["f-akceptacja"]))
        self.assertEqual(data["id"], "f-akceptacja")
        self.assertEqual(data["executionPlatform"], "Camunda Cloud")
        comps = data["components"]
        self.assertEqual(comps[0], {"type": "text", "id": "Text_1", "text": "## Propozycja: {{tytul}}"})
        zgoda = comps[1]
        self.assertEqual(zgoda["id"], "Field_zgoda")
        self.assertEqual(zgoda["key"], "zgoda")
        self.assertEqual(zgoda["type"], "radio")
        self.assertEqual(zgoda["validate"], {"required": True})
        self.assertEqual(zgoda["values"], [{"label": "zgoda na zakup", "value": True},
                                           {"label": "odmowa", "value": False}])
        self.assertNotIn("validate", comps[2])

    def test_number_and_select(self):
        comps = json.loads(forms.export_form(self.doc.forms["f-propozycja"]))["components"]
        by_key = dict((c.get("key"), c) for c in comps)
        self.assertEqual(by_key["cena"]["validate"], {"required": True, "min": 0})
        self.assertEqual(by_key["rodzaj"]["defaultValue"], "drukowana")
        self.assertEqual([v["value"] for v in by_key["rodzaj"]["values"]], ["drukowana", "e-book"])

    def test_options_conversion(self):
        self.assertEqual(forms.parse_options("1:jeden, 2.5:dwa i pół, x"),
                         [{"label": "jeden", "value": 1}, {"label": "dwa i pół", "value": 2.5},
                          {"label": "x", "value": "x"}])

    def test_checks(self):
        text = ('tok 0.1 "t"\n\nform f\n  field a "A" type=slider\n  field b "B" type=select\n'
                '  field c "C" type=radio options="1:x"\n')
        doc, diags = parse(text)
        self.assertEqual(diags, [])
        codes = [(c, line) for c, line, _ in forms.check_form(doc.forms["f"])]
        self.assertEqual(codes, [("M001", 4), ("M002", 5)])


if __name__ == "__main__":
    unittest.main()
