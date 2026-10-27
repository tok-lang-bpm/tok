import os
import random
import tempfile
import unittest

from tok.parser import parse
from tok import sim

HERE = os.path.dirname(os.path.abspath(__file__))
EXAMPLES = os.path.join(HERE, "..", "..", "examples")


def doc(text):
    d, diags = parse(text)
    errors = [x for x in diags if x.severity == "error"]
    assert not errors, [x.message() for x in errors]
    return d


class DistributionTests(unittest.TestCase):
    def test_durations_and_numbers(self):
        self.assertEqual(sim.parse_dist("30m").sample(random.Random(1)), 1800.0)
        self.assertEqual(sim.parse_dist("1.5d").sample(random.Random(1)), 129600.0)
        self.assertEqual(sim.parse_dist("12").sample(random.Random(1)), 12.0)
        self.assertEqual(sim.parse_dist("fixed(2h)").sample(random.Random(1)), 7200.0)

    def test_means(self):
        cases = {"unif(10m, 20m)": 900.0, "tri(1h, 2h, 6h)": 3 * 3600.0, "exp(2h)": 7200.0,
                 "lnorm(30m, 10m)": 1800.0, "norm(100, 5)": 100.0}
        for text, mean in cases.items():
            rng = random.Random(42)
            d = sim.parse_dist(text)
            xs = [d.sample(rng) for _ in range(20000)]
            self.assertAlmostEqual(sum(xs) / len(xs) / mean, 1.0, delta=0.03, msg=text)

    def test_lnorm_sd(self):
        rng = random.Random(5)
        d = sim.parse_dist("lnorm(10, 4)")
        xs = [d.sample(rng) for _ in range(40000)]
        m = sum(xs) / len(xs)
        sd = (sum((x - m) ** 2 for x in xs) / len(xs)) ** 0.5
        self.assertAlmostEqual(sd, 4.0, delta=0.2)

    def test_bern_cat(self):
        rng = random.Random(3)
        b = sim.parse_dist("bern(0.25)")
        self.assertAlmostEqual(sum(b.sample(rng) for _ in range(20000)) / 20000, 0.25, delta=0.015)
        c = sim.parse_dist("cat(drukowana=0.7, e-book=0.3)")
        xs = [c.sample(rng) for _ in range(20000)]
        self.assertAlmostEqual(xs.count("e-book") / 20000.0, 0.3, delta=0.015)
        self.assertEqual(sim.parse_dist("cat(true=0.5, false=0.5)").labels, [True, False])

    def test_seeded_determinism(self):
        d = sim.parse_dist("tri(5m, 10m, 20m)")
        a = [d.sample(random.Random(9)) for _ in range(3)]
        b = [d.sample(random.Random(9)) for _ in range(3)]
        self.assertEqual(a, b)

    def test_errors(self):
        for bad in ("", "tri(3, 2, 1)", "unif(1h, 2)", "exp(0)", "cat(a=0.5)", "bern(2)", "gauss(1, 2)",
                    "unif(1)", "abc"):
            with self.assertRaises(sim.DistError, msg=bad):
                sim.parse_dist(bad)

    def test_assign_and_iso(self):
        pairs = sim.parse_assign("zgoda=bern(0.8); tryb=cat(a=0.5, b=0.5)")
        self.assertEqual([n for n, _ in pairs], ["zgoda", "tryb"])
        self.assertEqual(sim.iso_duration("P7D"), 7 * 86400)
        self.assertEqual(sim.iso_duration("PT2H30M"), 9000)
        self.assertEqual(sim.iso_duration("P1W"), 7 * 86400)
        self.assertIsNone(sim.iso_duration("2027-01-01"))


LINEAR = """tok 0.1 "Linear"
start s
task a "A" by=r duration="5m"
end e
flow s -> a -> e
simulate cases=3000 seed=3 arrival="exp(10m)"
resource r count=1
"""


class ModelTests(unittest.TestCase):
    def test_single_server_queue(self):
        res = sim.simulate(doc(LINEAR))
        k = res.kpi()
        self.assertEqual(k["cases"]["finished"], 3000)
        util = k["resources"]["r"]["utilization"]
        self.assertAlmostEqual(util, 0.5, delta=0.04)
        # M/D/1 with rho = 0.5: mean wait = rho * D / (2 (1 - rho)) = 2.5 min
        self.assertAlmostEqual(k["activities"]["a"]["mean_wait_s"] / 60.0, 2.5, delta=0.6)
        self.assertAlmostEqual(k["activities"]["a"]["mean_processing_s"], 300.0, delta=1e-6)

    def test_xor_with_probabilities(self):
        d = doc("""tok 0.1
start s
xor g "Q?"
task a
task b
end e1
end e2
flow s -> g
flow g -> a "yes" p=0.3
flow g -> b "no" p=0.7
flow a -> e1
flow b -> e2
simulate cases=4000 seed=11 arrival="1h"
""")
        k = sim.simulate(d).kpi()
        self.assertAlmostEqual(k["cases"]["by_end_event"]["e1"] / 4000.0, 0.3, delta=0.025)

    def test_xor_with_conditions(self):
        d = doc("""tok 0.1
start s
script c expr="cena + 10" result=koszt
xor g "Drogo?"
end drogo
end tanio
flow s -> c -> g
flow g -> drogo "tak" when="koszt > 100"
flow g -> tanio "nie" default
simulate cases=2000 seed=2 arrival="1h"
variable cena "unif(0, 200)"
""")
        r = sim.simulate(d)
        for c in r.cases:
            v = c["variables"]
            self.assertEqual(c["end_event"], "drogo" if v["cena"] + 10 > 100 else "tanio")

    def test_and_split_join(self):
        d = doc("""tok 0.1
start s
and fork
task a duration="1h"
task b duration="2h"
and join
end e
flow s -> fork
flow fork -> a
flow fork -> b
flow a -> join
flow b -> join
flow join -> e
simulate cases=20 seed=1 arrival="3h"
""")
        r = sim.simulate(d)
        for c in r.cases:
            self.assertEqual(c["end_event"], "e")
            self.assertAlmostEqual(c["cycle_time_s"], 7200.0)
        self.assertEqual(len(r.log), 40)

    def test_or_split_join(self):
        d = doc("""tok 0.1
start s
or fork
task a duration="1h"
task b duration="2h"
or join
end e
flow s -> fork
flow fork -> a p=1
flow fork -> b p=0
flow a -> join
flow b -> join
flow join -> e
simulate cases=10 seed=1 arrival="3h"
""")
        r = sim.simulate(d)
        for c in r.cases:
            self.assertEqual(c["end_event"], "e")
            self.assertAlmostEqual(c["cycle_time_s"], 3600.0)

    def test_boundary_timer_interrupts(self):
        d = doc("""tok 0.1
start s
user a "Akceptacja" by=k duration="3h"
boundary t "Termin" on=a timer=PT1H
end ok
end late
flow s -> a -> ok
flow t -> late
simulate cases=10 seed=4 arrival="5h"
resource k count=1
""")
        r = sim.simulate(d)
        k = r.kpi()
        self.assertEqual(k["cases"]["by_end_event"], {"late": 10})
        for c in r.cases:
            self.assertAlmostEqual(c["cycle_time_s"], 3600.0)
        self.assertTrue(all(row["status"] == "interrupted" for row in r.log))
        self.assertAlmostEqual(k["resources"]["k"]["utilization"], 10 * 3600.0 / k["horizon_s"], delta=1e-3)

    def test_noninterrupting_boundary(self):
        d = doc("""tok 0.1
start s
task a duration="3h"
boundary t on=a timer=PT1H noninterrupting
task remind duration="10m"
end e1
end e2
flow s -> a -> e1
flow t -> remind -> e2
simulate cases=5 seed=4 arrival="5h"
""")
        r = sim.simulate(d)
        self.assertEqual(len([x for x in r.log if x["activity"] == "remind"]), 5)
        for c in r.cases:
            self.assertAlmostEqual(c["cycle_time_s"], 3 * 3600.0)
            self.assertEqual(c["end_event"], "e1")

    def test_event_gateway_race_and_subprocess(self):
        d = doc("""tok 0.1
start s
sub w "Oczekiwanie"
  start ws
  eventgw g
  catch msg "Odpowiedź" message="odpowiedz" delay="2h"
  catch tm "Minęło 14 dni" timer=P14D
  end we1
  end we2
  flow ws -> g
  flow g -> msg
  flow g -> tm
  flow msg -> we1
  flow tm -> we2
end e
flow s -> w -> e
simulate cases=3 seed=1 arrival="1d"
""")
        r = sim.simulate(d)
        for c in r.cases:
            self.assertAlmostEqual(c["cycle_time_s"], 7200.0)
            self.assertEqual(c["end_event"], "e")
        self.assertEqual([x["activity"] for x in r.log], ["w", "w", "w"])

    def test_terminate_and_loop_limit(self):
        d = doc("""tok 0.1
start s
and fork
task a duration="1h"
task b duration="5h"
end stop terminate
end e
flow s -> fork
flow fork -> a
flow fork -> b
flow a -> stop
flow b -> e
simulate cases=2 seed=1 arrival="1d"
""")
        r = sim.simulate(d)
        for c in r.cases:
            self.assertEqual(c["end_event"], "stop")
            self.assertAlmostEqual(c["cycle_time_s"], 3600.0)
        loop = doc("""tok 0.1
start s
task a
xor g
end e
flow s -> a -> g
flow g -> a "again" p=1
flow g -> e "done" p=0
simulate cases=1
""")
        r = sim.simulate(loop)
        self.assertEqual(r.cases[0]["end_event"], "loop-limit")

    def test_determinism_and_output(self):
        a = sim.simulate(doc(LINEAR), cases=50, seed=8)
        b = sim.simulate(doc(LINEAR), cases=50, seed=8)
        self.assertEqual(a.log, b.log)
        c = sim.simulate(doc(LINEAR), cases=50, seed=9)
        self.assertNotEqual(a.log, c.log)
        tmp = tempfile.mkdtemp()
        a.write_csv(os.path.join(tmp, "log.csv"), os.path.join(tmp, "cases.csv"))
        with open(os.path.join(tmp, "log.csv"), encoding="utf-8") as f:
            self.assertEqual(f.readline().strip(), "case,activity,label,resource,enqueue,start,end,status")
        self.assertIn('"kpi"', a.to_json())
        self.assertEqual(a.log[0]["enqueue"], "2027-01-04T08:00:00")


class CheckTests(unittest.TestCase):
    def test_check_sim(self):
        d = doc("""tok 0.1
start s
task a by=nobody duration="tri(3, 2, 1)" assign="x=bern(1.5)"
xor g
xor h
task b
task c
end e
flow s -> a -> g
flow g -> h p=0.5
flow g -> b p=0.2
flow h -> b
flow h -> c
flow b -> e
flow c -> e
simulate arrival="exp(0)"
variable cena "unif(5, 1)"
""")
        codes = sorted(c for c, _l, _i, _a in sim.check_sim(d))
        self.assertEqual(codes, ["R002", "S001", "S001", "S001", "S001", "S002", "S003"])

    def test_example_parses_and_runs(self):
        with open(os.path.join(EXAMPLES, "kwerenda-symulacja.tok"), encoding="utf-8") as f:
            d, diags = parse(f.read())
        self.assertEqual([x.code for x in diags], [])
        self.assertEqual(sim.check_sim(d), [])
        k = sim.simulate(d).kpi()
        self.assertEqual(k["cases"]["finished"], 300)
        self.assertEqual(set(k["cases"]["by_end_event"]), {"odrzucony", "zakonczona"})


if __name__ == "__main__":
    unittest.main()
