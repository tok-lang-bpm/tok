"""Generate the Python-reference fixture for tests/pyrandom_parity.rs.

Usage (from any directory):
    python tok/rust/tok-sim/tests/gen_python_expected.py

Output: fixtures/pyrandom-PLATFORM.txt (windows, linux or macos), one call of
``random.Random`` per line. The Rust test replays the lines in order on one
generator and compares every result. ``log``, ``exp``, ``sin`` and ``cos`` come
from the platform C library, so results that pass through them (gauss,
normalvariate, lognormvariate, expovariate and the lognormal parameters below)
can differ in the last bit between platforms; run the script on every platform
the tests run on and commit each file.

Line format (floats are written with ``repr``, the shortest round-trip form):
    python VERSION                  informational header
    const NAME VALUE                module constant of ``random``
    seed INT                        rng.seed(INT) (the first one creates the generator)
    seedf TEXT                      rng.seed(int(float(TEXT))), as ``sim.py`` derives its seed
    u32 VALUE                       rng.getrandbits(32)
    random VALUE                    rng.random()
    uniform A B VALUE               rng.uniform(A, B)
    triangular LOW HIGH MODE VALUE  rng.triangular(LOW, HIGH, MODE); MODE may be None
    gauss MU SIGMA VALUE            rng.gauss(MU, SIGMA)
    normalvariate MU SIGMA VALUE    rng.normalvariate(MU, SIGMA)
    lognormvariate MU SIGMA VALUE   rng.lognormvariate(MU, SIGMA)
    expovariate LAMBD VALUE         rng.expovariate(LAMBD)
"""

import io
import math
import os
import random
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
PLATFORM = {"win32": "windows", "darwin": "macos"}.get(sys.platform, "linux")
OUT = os.path.join(HERE, "fixtures", "pyrandom-%s.txt" % PLATFORM)

SEEDS = [
    0, 1, 2, 42, 12345, -1, -7, -42,
    2**31 - 1, 2**32 - 1, 2**32, 2**32 + 5, 2**64 - 1, 2**64,
    2**70, -(2**70), 123456789012345678901234567890, 2**623 + 17,
]

FLOAT_SEEDS = ["1", "-7.9", "0.5", "2.5e15", "1e30", "-1e30", "1.7976931348623157e308"]

UNIFORM = [
    (0.0, 1.0), (60.0, 300.0), (300.0, 1200.0), (-5.0, 5.0), (2.5, 2.5),
    (-1e6, 1e6), (0.001, 0.002), (5.0, 1.0),
]

TRIANGULAR = [
    (0.0, 1.0, None), (300.0, 1200.0, 600.0), (300.0, 1200.0, 300.0),
    (300.0, 1200.0, 1200.0), (5.0, 5.0, 5.0), (5.0, 5.0, None), (-3.0, 7.0, 0.5),
    (0.1, 0.7, 0.3), (10.0, 2.0, 4.0), (0.0, 86400.0, 3600.0),
]

GAUSS = [(0.0, 1.0), (600.0, 120.0), (-5.0, 0.5), (100.0, 0.0), (1e6, 3e5), (0.3, 1e-9)]

# (mean, sd) of lnorm() in sim.py, converted with the same formulas.
LNORM = [(600.0, 120.0), (1.0, 1.0), (3600.0, 7200.0), (10.0, 0.0), (0.5, 0.01), (1e5, 1e6)]

EXP_MEANS = [1.0, 0.5, 300.0, 3600.0, 1e-3, 1e6, 7.0 / 3.0]


def lnorm_params(mean, sd):
    s2 = math.log(1.0 + (sd * sd) / (mean * mean))
    return math.log(mean) - s2 / 2.0, math.sqrt(s2)


class Writer:
    def __init__(self):
        self.lines = []
        self.values = 0
        self.rng = None

    def seed(self, n, text=None):
        if self.rng is None:
            self.rng = random.Random(n)
        else:
            self.rng.seed(n)
        self.lines.append("seed %d" % n if text is None else "seedf %s" % text)

    def call(self, name, *args):
        if name == "u32":
            v = self.rng.getrandbits(32)
            self.lines.append("u32 %d" % v)
        else:
            v = getattr(self.rng, name)(*args)
            parts = [name] + ["None" if a is None else repr(float(a)) for a in args] + [repr(v)]
            self.lines.append(" ".join(parts))
        self.values += 1


def blocks(w, n):
    """The fixed blocks run for every seed ``n``."""
    w.seed(n)
    for _ in range(8):
        w.call("u32")
    w.seed(n)
    for _ in range(1300):     # more than four state regenerations of 624 words
        w.call("random")
    w.seed(n)
    for a, b in UNIFORM:
        for _ in range(3):
            w.call("uniform", a, b)
    w.seed(n)
    for low, high, mode in TRIANGULAR:
        for _ in range(4):
            w.call("triangular", low, high, mode)
    w.seed(n)
    for mu, sigma in GAUSS:
        for _ in range(3):    # odd count: the cached second value crosses parameter sets
            w.call("gauss", mu, sigma)
    w.call("gauss", 0.0, 1.0)  # fills the cache
    w.call("random")
    w.call("gauss", 0.0, 1.0)  # served from the cache
    w.call("gauss", 0.0, 1.0)
    w.call("normalvariate", 0.0, 1.0)
    w.call("gauss", 0.0, 1.0)
    w.call("uniform", 0.0, 1.0)
    w.call("gauss", 5.0, 2.0)
    w.call("gauss", 0.0, 1.0)  # leaves a cached value behind
    w.seed(n)                  # seeding clears the cache
    w.call("gauss", 0.0, 1.0)
    w.call("gauss", 0.0, 1.0)
    w.seed(n)
    for mu, sigma in GAUSS:
        for _ in range(3):
            w.call("normalvariate", mu, sigma)
    w.seed(n)
    for mean, sd in LNORM:
        mu, sigma = lnorm_params(mean, sd)
        for _ in range(3):
            w.call("lognormvariate", mu, sigma)
    w.seed(n)
    for mean in EXP_MEANS:
        for _ in range(3):
            w.call("expovariate", 1.0 / mean)


def mixed(w, n, meta_seed, count):
    """A long interleaved sequence of all methods, chosen by an independent generator."""
    meta = random.Random(meta_seed)
    w.seed(n)
    for _ in range(count):
        op = meta.choice(["random", "uniform", "triangular", "gauss", "gauss", "normalvariate",
                          "lognormvariate", "expovariate", "u32"])
        a = round(meta.uniform(-100.0, 100.0), 3)
        b = round(a + meta.uniform(0.0, 500.0), 3)
        if op in ("random", "u32"):
            w.call(op)
        elif op == "uniform":
            w.call(op, a, b)
        elif op == "triangular":
            mode = None if meta.random() < 0.2 else round(meta.uniform(a, b), 3)
            w.call(op, a, b, mode)
        elif op in ("gauss", "normalvariate"):
            w.call(op, a, round(meta.uniform(0.0, 50.0), 3))
        elif op == "lognormvariate":
            mu, sigma = lnorm_params(round(meta.uniform(1.0, 5000.0), 2), round(meta.uniform(0.0, 3000.0), 2))
            w.call(op, mu, sigma)
        else:
            w.call(op, 1.0 / round(meta.uniform(0.01, 5000.0), 3))


def main():
    w = Writer()
    w.lines.append("python %s %s" % (sys.version.split()[0], PLATFORM))
    w.lines.append("const NV_MAGICCONST %r" % random.NV_MAGICCONST)
    w.lines.append("const TWOPI %r" % random.TWOPI)
    for n in SEEDS:
        blocks(w, n)
    for text in FLOAT_SEEDS:
        w.seed(int(float(text)), text)
        for _ in range(4):
            w.call("u32")
        w.call("random")
        w.call("gauss", 0.0, 1.0)
    for i, n in enumerate([1, 42, 2**70, -7]):
        mixed(w, n, 1000 + i, 2500)
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with io.open(OUT, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(w.lines) + "\n")
    print("values: %d, lines: %d" % (w.values, len(w.lines)))


if __name__ == "__main__":
    main()
