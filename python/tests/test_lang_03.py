"""Language 0.3: ad-hoc sub-processes, compensation, catch-all events and the executable layer."""

import datetime
import os
import unittest

from tok import feel, sim
from tok.bpmn_export import semantic_xml
from tok.bpmn_import import import_bpmn
from tok.check import check
from tok.parser import parse
from tok.printer import fmt

HERE = os.path.dirname(os.path.abspath(__file__))
CASES = os.path.join(HERE, "..", "..", "conformance", "cases")


def load(name):
    with open(os.path.join(CASES, name), encoding="utf-8") as f:
        return f.read()


def codes(text, **kw):
    doc, diags = parse(text)
    if not any(d.severity == "error" for d in diags):
        diags = diags + check(doc, **kw)
    return sorted(d.code for d in diags)


def xml(text):
    doc, diags = parse(text)
    assert not [d for d in diags if d.severity == "error"], diags
    return semantic_xml(doc)


class Header(unittest.TestCase):
    def test_versions(self):
        self.assertNotIn("P006", codes('tok 0.1\nstart s\n'))
        self.assertNotIn("P006", codes('tok 0.3\nstart s\n'))
        self.assertIn("P006", codes('tok 0.2\nstart s\n'))

    def test_language_01_documents_unchanged(self):
        text = 'tok 0.1 "T"\nstart s\ntask a "A"\nend e\nflow s -> a -> e\n'
        self.assertIn('exporterVersion="0.1"', xml(text))
        self.assertEqual(codes(text), [])


class AdHoc(unittest.TestCase):
    def test_block_and_print(self):
        doc, diags = parse(load("20-adhoc.tok"))
        self.assertEqual(diags, [])
        self.assertEqual(doc.nodes["opracowanie"].kind, "adhoc")
        self.assertEqual(doc.nodes["opracowanie"].children, ["inwentarz", "skany", "konserwacja"])
        self.assertEqual(fmt(parse(fmt(doc))[0]), fmt(doc))

    def test_export(self):
        x = xml(load("20-adhoc.tok"))
        self.assertIn('<bpmn:adHocSubProcess cancelRemainingInstances="false" id="opracowanie"', x)
        self.assertIn('<zeebe:adHoc activeElementsCollection="=wybrane" />', x)
        self.assertIn('ordering="Sequential"', x)
        # completionCondition is the last child of the ad-hoc sub-process
        block = x[x.index('id="opracowanie"'):x.index("</bpmn:adHocSubProcess>")]
        self.assertTrue(block.rstrip().endswith("</bpmn:completionCondition>"))

    def test_r050(self):
        self.assertIn("R050", codes('tok 0.3\nstart s\nadhoc a\n  start s2\n  end e2\nend e\nflow s -> a -> e\n'
                                    'flow s2 -> e2\n'))


class ExecutableLayer(unittest.TestCase):
    def test_multi_instance(self):
        x = xml(load("19-multi-instance.tok"))
        self.assertIn('<zeebe:loopCharacteristics inputCollection="=pozycje" inputElement="x" '
                      'outputCollection="wyniki" outputElement="=y" />', x)
        self.assertIn(">=numberOfCompletedInstances &gt;= 2</bpmn:completionCondition>", x)

    def test_call_activity_and_bindings(self):
        x = xml(load("21-call-activity.tok"))
        self.assertIn('<zeebe:calledElement bindingType="versionTag" businessId="=numer" '
                      'processId="wysylka" propagateAllChildVariables="false" '
                      'propagateAllParentVariables="false" versionTag="v2" />', x)
        self.assertIn('processId="=nazwaProcesu"', x)
        self.assertNotIn('calledElement="=nazwaProcesu"', x)
        self.assertIn('<zeebe:calledDecision bindingType="deployment" decisionId="tryb"', x)
        self.assertIn('<zeebe:formDefinition bindingType="versionTag" formId="f-start" versionTag="v3" />', x)

    def test_mappings(self):
        x = xml(load("22-mappings.tok"))
        self.assertIn('<zeebe:input source="=priorytet &gt; 70" target="pilne" />', x)
        self.assertIn('<zeebe:output source="=konto" target="nadawca.iban" />', x)
        self.assertIn('<zeebe:output source="=kto" target="inicjator" />', x)

    def test_user_task(self):
        x = xml(load("23-user-task.tok"))
        self.assertIn('<zeebe:assignmentDefinition assignee="=wniosek.przelozony" candidateGroups="kierownicy, '
                      'sekretariat" candidateUsers="anna, jan" />', x)
        self.assertIn('dueDate="2027-01-15T12:00:00+01:00"', x)
        self.assertIn('<zeebe:priorityDefinition priority="70" />', x)

    def test_jobs_and_connector(self):
        x = xml(load("24-jobs.tok"))
        self.assertIn('<zeebe:taskDefinition retries="5" type="email" />', x)
        self.assertIn('<zeebe:header key="szablon" value="odmowa" />', x)
        self.assertIn('<zeebe:taskDefinition type="odpowiedz" />', x)
        c = xml(load("25-connector.tok"))
        self.assertIn('<zeebe:taskDefinition type="io.camunda:http-json:1" />', c)
        self.assertIn('<zeebe:input source="noAuth" target="authentication.type" />', c)
        self.assertIn('<zeebe:header key="resultVariable" value="rekord" />', c)

    def test_compensation_and_catch_all(self):
        x = xml(load("26-compensation.tok"))
        self.assertIn('<bpmn:boundaryEvent attachedToRef="rezerwacja" id="rezerwacja-compensate">', x)
        self.assertIn('<bpmn:association associationDirection="One" id="rezerwacja-compensate-assoc" '
                      'sourceRef="rezerwacja-compensate" targetRef="zwolnienie" />', x)
        self.assertIn('activityRef="rezerwacja"', x)
        y = xml(load("27-catch-all.tok"))
        self.assertIn('<bpmn:errorEventDefinition id="blad-def" />', y)
        self.assertIn('<bpmn:escalationEventDefinition id="eskal-def" />', y)

    def test_conditional_listeners_version(self):
        x = xml(load("28-conditional.tok"))
        self.assertIn('<zeebe:conditionalFilter variableEvents="create, update" />', x)
        y = xml(load("29-listeners.tok"))
        self.assertIn('<zeebe:executionListener eventType="end" retries="5" type="po" />', y)
        self.assertIn('<zeebe:taskListener eventType="creating" retries="2" type="utworzenie" />', y)
        z = xml(load("30-version-tag.tok"))
        self.assertIn('<zeebe:versionTag value="v2.1" />', z)
        self.assertIn('<zeebe:versionTag value="2027-01" />', z)

    def test_listener_order_canonical(self):
        doc, _ = parse(load("29-listeners.tok"))
        self.assertIn('listeners="start: wejscie; creating: utworzenie (2); completing: zakonczenie"', fmt(doc))

    def test_not_executable_is_ignored(self):
        text = load("31-levels.tok")
        self.assertEqual(codes(text).count("R055"), 7)
        x = xml(text)
        self.assertNotIn("zeebe", x)
        self.assertNotIn("cancelRemainingInstances", x)

    def test_levels(self):
        text = load("19-multi-instance.tok")
        self.assertIn("R040", codes(text, level="analytic"))
        self.assertIn("R040", codes(text, level="basic"))
        self.assertNotIn("R040", codes(text))

    def test_error_codes(self):
        got = set(codes(load("32-err-03.tok")))
        for c in ("R050", "R051", "R052", "R053", "R054", "C022", "C090", "C091"):
            self.assertIn(c, got)

    def test_round_trip(self):
        for n in range(19, 32):
            name = [f for f in os.listdir(CASES) if f.startswith("%d-" % n)][0]
            x1 = xml(load(name))
            doc, warnings = import_bpmn(x1)
            self.assertEqual(warnings, [], name)
            self.assertEqual(semantic_xml(parse(fmt(doc))[0]), x1, name)


class Feel(unittest.TestCase):
    def test_temporal_helpers(self):
        self.assertIsNone(feel.check_expr('now() + duration("P1D")'))
        self.assertEqual(feel.parse_duration("P1DT2H"), datetime.timedelta(days=1, hours=2))
        self.assertIsNone(feel.parse_duration("P1Y"))


class Simulation(unittest.TestCase):
    TEXT = '''tok 0.3 "Symulacja"

start s "Start"
script podw "Podwojenie" parallel expr="x * 2" result=y collection="[1, 2, 3]" item=x output=wyniki output-item="y" duration="10m"
user ocena "Ocena" sequential collection="[\\"a\\", \\"b\\", \\"c\\"]" item=r until="numberOfCompletedInstances >= 2" duration="1h"
adhoc op "Opracowanie" active="[\\"inw\\"]"
  task inw "Inwentarz" duration="2h"
  task sk "Skany" duration="5h"
service rez "Rezerwacja" duration="5m" compensate=zw
service zw "Zwolnienie" duration="7m" compensation
throw cof "Cofnięcie" compensate activity=rez
end e "Koniec"

flow s -> podw -> ocena -> op -> rez -> cof -> e

simulate cases=1 seed=1
'''

    def test_constructs(self):
        doc, diags = parse(self.TEXT)
        self.assertEqual(diags, [])
        r = sim.simulate(doc)
        acts = [row["activity"] for row in r.log]
        self.assertEqual(acts.count("podw"), 3)
        self.assertEqual(acts.count("ocena"), 2)
        self.assertIn("inw", acts)
        self.assertNotIn("sk", acts)
        self.assertIn("zw", acts)
        self.assertEqual(r.cases[0]["variables"]["wyniki"], [2, 4, 6])
        self.assertEqual(r.cases[0]["end_event"], "e")
        self.assertEqual(r.cases[0]["cycle_time_s"], 15720.0)


if __name__ == "__main__":
    unittest.main()
