"""Tests for BPMN 2.0 XML -> tok import.

Run from the python/ folder:  PYTHONPATH=src python -m unittest discover -s tests
"""

import glob
import os
import unittest

from tok import bpmnlib
from tok.bpmn_export import export_tree, semantic_xml
from tok.bpmn_import import import_bpmn
from tok.parser import parse
from tok.printer import fmt

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.normpath(os.path.join(HERE, "..", ".."))
EXAMPLES = os.path.join(REPO, "examples")
# BPMN models of the companion course, when the repository is checked out next to it
IMPORT_CORPUS = os.path.join(REPO, "conformance", "import")

RICH = '''tok 0.1 "Kwerenda archiwalna" id=kwerenda-def

pool wnioskodawca "Wnioskodawca"
pool archiwum "Archiwum państwowe"
  lane kancelaria "Kancelaria"
    start wplyw "Wniosek wpłynął" message="Wniosek o kwerendę"
    user rejestracja "Rejestracja wniosku"
    data wniosek "Wniosek" state="zarejestrowany"
  lane archiwista "Archiwista"
    sub poszukiwania "Poszukiwania w zasobie"
      start s1
      manual przeglad "Przegląd inwentarzy" loop
      xor znaleziono "Czy znaleziono akta?"
      throw eskalacja "Sprawa trudna" escalation="Trudna kwerenda" code="ESK"
      end s2
      end s3 "Akta znalezione"
    boundary trudna "Sprawa trudna" on=poszukiwania escalation="Trudna kwerenda" code="ESK" noninterrupting
    task konsultacja "Konsultacja z kierownikiem" parallel
    end konsultowano
    eventgw czekanie "Oczekiwanie na opłatę"
    catch oplata "Opłata wniesiona" message="Opłata"
    catch termin "Minęło 14 dni" timer=P14D
    send odpowiedz "Wysłanie odpowiedzi" message="Odpowiedź"
    end koniec "Kwerenda zakończona"
    end anulowana "Kwerenda anulowana" terminate
    eventsub wycofanie "Wycofanie wniosku"
      start w1 "Wniosek wycofany" message="Wycofanie" noninterrupting
      end w2
    store ewidencja "Ewidencja kwerend"

flow wplyw -> rejestracja -> poszukiwania -> czekanie
flow s1 -> przeglad -> znaleziono
flow znaleziono -> s3 "tak" when="liczba > 0"
flow znaleziono -> eskalacja "nie" default
flow eskalacja -> s2
flow trudna -> konsultacja -> konsultowano
flow czekanie -> oplata
flow czekanie -> termin
flow oplata -> odpowiedz -> koniec
flow termin -> anulowana
flow w1 -> w2

message wnioskodawca -> wplyw "Wniosek o kwerendę"
message odpowiedz -> wnioskodawca "Odpowiedź"

writes rejestracja wniosek
reads przeglad ewidencja
writes rejestracja ewidencja

note n1 "Podstawa: regulamin udostępniania" on=rejestracja
'''


def language_version(path):
    """The language version in the header of a tok document (None without a header)."""
    with open(path, encoding="utf-8") as f:
        for line in f.read().lstrip("\ufeff").splitlines():
            words = line.split("#", 1)[0].split()
            if words:
                return words[1] if words[0] == "tok" and len(words) > 1 else None
    return None


def roundtrip(text):
    d1, diags = parse(text)
    x1 = semantic_xml(d1)
    d2, warnings = import_bpmn(x1)
    t2 = fmt(d2)
    d3, diags3 = parse(t2)
    return x1, semantic_xml(d3), diags3, warnings, t2


class RoundTrip(unittest.TestCase):
    def check(self, text, name):
        x1, x2, diags, warnings, t2 = roundtrip(text)
        self.assertEqual([d.code for d in diags], [], "%s: printed text does not parse:\n%s" % (name, t2))
        self.assertEqual(x1, x2, "%s: semantic XML differs after tok -> XML -> tok" % name)

    def test_examples(self):
        # the examples of languages 0.1 and 0.3, which this implementation reads
        files = [p for p in sorted(glob.glob(os.path.join(EXAMPLES, "*.tok"))) if language_version(p) in ("0.1", "0.3", None)]
        self.assertTrue(files)
        for path in files:
            with open(path, encoding="utf-8") as f:
                self.check(f.read(), os.path.basename(path))

    def test_rich_document(self):
        self.check(RICH, "rich")

    def test_derived_ids_are_not_printed(self):
        _, _, _, _, text = roundtrip(RICH)
        self.assertNotIn(" id=f-", text)
        self.assertIn("flow wplyw -> rejestracja -> poszukiwania -> czekanie", text)

    def test_flow_ids_without_keep(self):
        doc, _ = parse(RICH)
        xml = semantic_xml(doc).replace('id="f-wplyw-rejestracja"', 'id="Flow_1"')
        kept, _ = import_bpmn(xml)
        self.assertIn("id=Flow_1", fmt(kept))
        dropped, _ = import_bpmn(xml, keep_ids=False)
        self.assertNotIn("Flow_1", fmt(dropped))


class ForeignIds(unittest.TestCase):
    def test_ids_are_converted_and_remapped(self):
        xml = '''<?xml version="1.0" encoding="UTF-8"?>
<bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL" id="D" targetNamespace="x">
  <bpmn:process id="P.1" isExecutable="false">
    <bpmn:startEvent id="start.1" name="Start"/>
    <bpmn:task id="zadanie ą" name="Zadanie"/>
    <bpmn:endEvent id="1end"/>
    <bpmn:complexGateway id="cx"/>
    <bpmn:sequenceFlow id="a" sourceRef="start.1" targetRef="zadanie ą"/>
    <bpmn:sequenceFlow id="b" sourceRef="zadanie ą" targetRef="1end"/>
    <bpmn:sequenceFlow id="c" sourceRef="zadanie ą" targetRef="cx"/>
  </bpmn:process>
</bpmn:definitions>'''
        doc, warnings = import_bpmn(xml)
        text = fmt(doc)
        _, diags = parse(text)
        self.assertEqual([d.code for d in diags], [], text)
        self.assertIn("start_1", text)
        self.assertIn("zadanie_a", text)
        self.assertIn("_1end", text)
        self.assertTrue(any("complexGateway" in w for w in warnings))
        self.assertTrue(any("dropped" in w for w in warnings))


def course_models():
    """BPMN models drawn for the course Modelowanie procesów informacyjnych (conformance/import)."""
    return sorted(glob.glob(os.path.join(IMPORT_CORPUS, "*.bpmn")))


class CourseModels(unittest.TestCase):
    """Every model of the import corpus imports, prints, re-parses and re-exports without BPMN errors."""

    def test_import_course_models(self):
        files = course_models()
        if not files:
            self.skipTest("import corpus not available")
        stats = {"files": 0, "warnings": 0}
        for path in files:
            with self.subTest(model=os.path.basename(path)):
                doc, warnings = import_bpmn(path)
                text = fmt(doc)
                doc2, diags = parse(text)
                self.assertEqual([d.code for d in diags], [], text)
                self.assertEqual(text, fmt(doc2), "fmt is not stable after import")
                m = bpmnlib.Model(export_tree(doc2))
                errors = [i for i in bpmnlib.validate(m) if i["poziom"] == "błąd"]
                expected_broken = path.endswith("-z-bledem.bpmn")
                if not expected_broken:
                    self.assertEqual(errors, [], "%s:\n%s" % (path, text))
                stats["files"] += 1
                stats["warnings"] += len(warnings)
        self.assertEqual(stats["files"], len(files))


if __name__ == "__main__":
    unittest.main()
