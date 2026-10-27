"""Generate the Python-reference fixtures of `tok sim` for tests/sim_parity.rs and
tok-cli/tests/sim_parity.rs.

Usage (from any directory, Python 3.12 or newer):
    PYTHONPATH=tok/python/src python tok/rust/tok-sim/tests/gen_python_sim.py [EXTRA_TOK_DIR ...]

Inputs: every document of tok/examples/*.tok, tok/conformance/cases/*.tok (copied as
conformance-NAME.tok), the course examples (../tok-course/przyklady next to the tok
repository, or EXTRA_TOK_DIR) and the synthetic models below (sim-*.tok), which cover every
distribution, timers, boundary and intermediate events, event-based gateways, resources with
queues, probabilities and conditions on flows, decisions, scripts, assign=, multi-instance
and ad-hoc activities, compensation, sub-processes, terminate, loop-limit and deadlock, and
invalid parameters.

Outputs in fixtures/sim/:
    NAME.tok              input copies (the same on every platform)
    cases-PLATFORM.json   `tok sim` runs (PLATFORM: windows, linux or macos): arguments,
                          environment, standard output, standard error, exit code and the CSV
                          files written. An exception the command line does not catch is
                          recorded as the last line of its traceback, exit code 1. In the
                          arguments, @OUT@ stands for a temporary folder.

The simulator draws from `random.Random`, whose gauss, lognormvariate and expovariate go
through the C library's log, exp, sin and cos; their last bit differs between platforms, so
run this script on every platform the tests run on (see rust/DEPENDENCIES.md). Python 3.12
changed sum() of floats to compensated summation, which the mean cycle time uses; the Rust
engine follows 3.12 and newer.
"""

import glob
import io
import json
import os
import shutil
import sys
import tempfile
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
TOK = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(TOK, "python", "src"))

from tok import cli  # noqa: E402
from tok.parser import parse  # noqa: E402

FIX = os.path.join(HERE, "fixtures", "sim")
COURSE = os.path.normpath(os.path.join(TOK, "..", "tok-course", "przyklady"))
PLATFORM = {"win32": "windows", "darwin": "macos"}.get(sys.platform, "linux")
OUT = "@OUT@"

# synthetic models (@BIG@: a duration of 10**339 seconds, an infinity as a float)
SYNTHETIC = {
    'sim-dists.tok': r'''tok 0.3 "Rozkłady"

start s "Start"
task a "Stała" duration="15m"
task b "Liczba" duration="90"
task c "Jednostajny" duration="unif(1m, 5m)"
task d "Trójkątny" duration="tri(5m, 10m, 20m)"
task e "Normalny" duration="norm(1h, 30m)"
task f "Logarytmiczno-normalny" duration="lnorm(2h, 1h)"
task g "Wykładniczy" duration="exp(45m)"
task h "Bernoulli" duration="bern(0.5)"
task i "Kategorie" duration="cat(10=0.25, 600=0.5, 3600.5=0.25)"
task j "Półtora dnia" duration="fixed(1.5d)"
task k "Sekundy" duration="tri(0s, 0s, 10s)"
task l "Zero" duration="unif(0m, 0m)"
task m "Ujemny" duration="norm(-5, 2)"
task n "Bez czasu"
end z "Koniec"

flow s -> a -> b -> c -> d -> e -> f -> g -> h -> i -> j -> k -> l -> m -> n -> z

simulate cases=20 seed=11 arrival="unif(10m, 2h)" start="2027-03-01T09:30"
variable cena "unif(20, 300)"
variable rodzaj "cat(drukowana=0.7, e-book=0.3)"
variable zgoda "bern(0.8)"
variable odchylenie "norm(100, 15)"
variable dochod "lnorm(10, 4)"
variable przerwa "exp(3)"
variable ocena "tri(1, 2, 6)"
variable stala "42"
variable czas "2h"
variable flaga "cat(true=0.5, false=0.5)"
variable liczba "cat(1=0.2, 2.5=0.3, -3=0.5)"
variable tekst "cat(\"tak\"=0.5, nie=0.5)"
variable cyfry "١٢"
''',
    'sim-queue.tok': r'''tok 0.1 "Kolejki"

start s "Wniosek"
and f
user a "Przyjęcie, rejestracja" by=r1 duration="exp(20m)"
user b "Analiza \"wstępna\"" by=r2 duration="tri(10m, 30m, 1h)"
and j
user c "Decyzja" by=r1 duration="10m"
service d "Archiwizacja" by=r3 duration="1m"
end e "Załatwiony"

flow s -> f
flow f -> a
flow f -> b
flow a -> j
flow b -> j
flow j -> c -> d -> e

simulate cases=40 seed=5 arrival="exp(15m)"
resource r1 "Referent" count=1
resource r2 "Analitycy" count=2.7
resource r3 count=1e1
''',
    'sim-blocked.tok': r'''tok 0.1 "Brak wykonawców"

start s
user a "Czeka na wykonawcę" by=nikt duration="1h"
end e

flow s -> a -> e

simulate cases=5 arrival="30m"
resource nikt "Nikt" count=0
''',
    'sim-gateways.tok': r'''tok 0.3 "Bramki"

start s "Start"
script k "Koszt" expr="cena + 10" result=koszt
xor g "Drogo?"
task drogo "Akceptacja" duration="1h"
task tanio "Automat" duration="5m"
task pomin "Pominięcie"
xor gj
or o "Zakres"
task x "Część X" duration="2h"
task y "Część Y" duration="3h"
task w "Część W" duration="30m"
or oj "Zbieranie"
xor p "Losowanie"
end e1 "Wariant 1"
end e2 "Wariant 2"
end e3 "Wariant 3"
xor bez "Bez warunków"
task b1 "Pierwszy"
task b2 "Drugi"
xor q "Błędne warunki"
xor u "Nazwy kluczowe"

flow s -> k -> g
flow g -> drogo "drogo" when="koszt > 150"
flow g -> tanio "tanio" when="koszt <= 150 and rodzaj = \"e-book\""
flow g -> pomin default
flow drogo -> gj
flow tanio -> gj
flow pomin -> gj
flow gj -> o
flow o -> x when="zgoda"
flow o -> y p=0.5
flow o -> w default
flow x -> oj
flow y -> oj
flow w -> oj
flow oj -> bez
flow bez -> b1
flow bez -> b2
flow b1 -> q
flow b2 -> q
flow q -> u when="koszt ** 2 > 100"
flow q -> u when="some > 100"
flow q -> p default
flow u -> p
flow p -> e1 p=0.2
flow p -> e2 p=0.3
flow p -> e3 p=0.5

simulate cases=30 seed=2 arrival="exp(2h)"
variable cena "unif(20, 300)"
variable rodzaj "cat(drukowana=0.7, e-book=0.3)"
variable zgoda "bern(0.4)"
variable some "unif(0, 200)"
''',
    'sim-events.tok': r'''tok 0.3 "Zdarzenia i czasomierze"

start s "Start"
catch t1 "Dzień" timer=P1D
catch t2 "Dwie i pół godziny" timer=PT2H30M
catch t3 "Cykl" timer="R3/PT1H"
catch t4 "Termin" timer="2027-01-05T12:00"
catch t5 "Miniony termin" timer="2026-12-31"
catch t6 "Niezrozumiały" timer="jutro"
catch t7 "Tydzień i ułamki" timer="P0.5WT1.5H"
catch t8 "Rok" timer=P1Y2M
catch m1 "Wiadomość" message="Odpowiedź" delay="exp(3h)"
catch m2 "Bez opóźnienia" message="Potwierdzenie"
catch t9 "Cykl bez okresu" timer="R2/x"
catch t10 "Data tygodniowa" timer="2027-W02-3T10:00"
eventgw g "Co pierwsze?"
catch r1 "Książka" message="Książka" delay="tri(1d, 2d, 5d)"
catch r2 "Termin minął" timer=P3D
receive r3 "Odbiór odpowiedzi" message="Odpowiedź" duration="exp(2d)"
catch r4 "Sygnał" signal="Stop"
end e1 "Książka"
end e2 "Termin"
end e3 "Odpowiedź"
end e4 "Sygnał"

flow s -> t1 -> t2 -> t3 -> t4 -> t5 -> t6 -> t7 -> t8 -> m1 -> m2 -> t9 -> t10 -> g
flow g -> r1
flow g -> r2
flow g -> r3
flow g -> r4
flow r1 -> e1
flow r2 -> e2
flow r3 -> e3
flow r4 -> e4

simulate cases=12 seed=4 arrival="exp(6h)"
''',
    'sim-eventgw-none.tok': r'''tok 0.1 "Bramka bez zdarzeń"

start s
eventgw g "Co pierwsze?"
catch a "Wiadomość A" message="A"
catch b "Sygnał B" signal="B"
end e1
end e2

flow s -> g
flow g -> a
flow g -> b
flow a -> e1
flow b -> e2

simulate cases=3
''',
    'sim-boundary.tok': r'''tok 0.3 "Zdarzenia brzegowe"

start s "Wniosek"
user o "Rozpatrzenie" by=urzednik duration="tri(1d, 3d, 7d)"
boundary b1 "Minęło 5 dni" on=o timer=P5D
boundary b2 "Przypomnienie" on=o timer="R3/P1D" noninterrupting
task przyp "Wysłanie przypomnienia" duration="10m"
end ep "Przypomniano"
end eu "Upłynął termin"
sub p "Uzupełnienie"
  start ps
  task p1 "Wezwanie" duration="2h"
  catch pc "Odpowiedź strony" message="Uzupełnienie" delay="exp(4d)"
  task p2 "Ocena uzupełnienia" duration="1h"
  end pe
  flow ps -> p1 -> pc -> p2 -> pe
boundary b3 "Brak odpowiedzi" on=p timer=P7D
boundary b4 "Telefon" on=p message="Telefon" delay="exp(2d)" noninterrupting
task tel "Notatka z rozmowy" duration="15m"
end et "Zanotowano"
end eb "Bez uzupełnienia"
end e "Załatwiony"

flow s -> o -> p -> e
flow b1 -> eu
flow b2 -> przyp -> ep
flow b3 -> eb
flow b4 -> tel -> et

simulate cases=25 seed=8 arrival="exp(1d)"
resource urzednik "Urzędnik" count=2
''',
    'sim-sub.tok': r'''tok 0.3 "Podprocesy i zakończenia"

start s "Start"
sub p "Kwerenda"
  start ps
  and pf
  task pa "Inwentarz" duration="tri(1h, 2h, 4h)"
  task pb "Akta" duration="exp(3h)"
  and pj
  xor px "Znaleziono?"
  end pe "Znaleziono"
  end pt "Przerwano" terminate
  flow ps -> pf
  flow pf -> pa
  flow pf -> pb
  flow pa -> pj
  flow pb -> pj
  flow pj -> px
  flow px -> pe "tak" p=0.7
  flow px -> pt "nie" p=0.3
sub pusty "Podproces bez startu" duration="20m"
  task pu "Wewnątrz"
xor g "Dalej?"
and f
task a "Krótka" duration="1h"
task b "Długa" duration="5h"
end stop "Przerwanie" terminate
end e "Koniec"
eventsub w "Wycofanie"
  start ws "Wycofano" message="Wycofanie"
  task wt "Zamknięcie"
  end we
  flow ws -> wt -> we
xor zly "Zła ścieżka"
and dj
end ed "Zakleszczenie"

flow s -> p -> pusty -> g
flow g -> f "równolegle" p=0.6
flow g -> zly "zła" p=0.4
flow f -> a
flow f -> b
flow a -> stop
flow b -> e
flow zly -> dj
flow dj -> ed

simulate cases=30 seed=6 arrival="exp(4h)"
''',
    'sim-loop.tok': r'''tok 0.1 "Pętla"

start s
task a "Poprawka"
xor g "Jeszcze raz?"
end e

flow s -> a -> g
flow g -> a "again" p=1
flow g -> e "done" p=0

simulate cases=2
''',
    'sim-rules.tok': r'''tok 0.3 "Decyzje i skrypty"

start s "Wniosek"
rule r1 "Limit" decision=limit duration="5m"
rule r2 "Rabat" decision=rabat result=wynik_rabatu
rule r3 "Kanały" decision=kanaly
rule r5 "Niejednoznaczna" decision=dwuznaczna
script k1 "Suma" expr="limit + wiek" result=suma
script k2 "Tekst" expr="string(wiek) + \" lat\""
script k3 "Błąd" expr="wiek ** 2" result=kwadrat
script k4 "Lista" expr="[wiek, limit, null]" result=lista
script k5 "Kontekst" expr="wynik_rabatu.rabat * 2" result=podwojony
script k6 "Czas" expr="duration(\"P1DT2H\")" result=okres
script k7 "Data" expr="date(\"2027-01-15\")" result=dzien
task z "Zgoda klienta" duration="1h" assign="zgoda=bern(0.7); kanal_pref=cat(sms=0.5, list=0.5); ocena=norm(3, 1)"
end e "Koniec"

flow s -> r1 -> r2 -> r3 -> r5 -> k1 -> k2 -> k3 -> k4 -> k5 -> k6 -> k7 -> z -> e

decision limit "Limit" hit=first
  | wiek: number | typ: string || limit: number |
  | < 18 | - || 5 |
  | >= 18 | "student","doktorant" || 15 |
  | >= 18 | "pracownik" || 30 |
  | - | - || 10 |

decision rabat "Rabat" hit=unique
  | kwota: number || rabat: number | opis: string |
  | < 100 || 0 | "brak" |
  | >= 100 || kwota * 0.05 | "5%" |

decision kanaly "Kanały" hit=collect
  | pilne: boolean || kanal: string |
  | true || "sms" |
  | - || "e-mail" |

decision dwuznaczna "Dwuznaczna" hit=unique
  | wiek: number || grupa: string |
  | >= 18 || "dorosły" |
  | [30..60] || "średni wiek" |
  | < 18 || "młody" |

simulate cases=15 seed=3 arrival="exp(1h)"
variable wiek "unif(10, 70)"
variable typ "cat(student=0.3, pracownik=0.3, inny=0.4)"
variable kwota "tri(10, 100, 400)"
variable pilne "bern(0.3)"
''',
    'sim-multi.tok': r'''tok 0.3 "Wiele instancji" executable

start s "Start"
script podw "Podwojenie" parallel expr="x * 2" result=y collection="[1, 2, 3]" item=x output=wyniki output-item="y" duration="10m"
user ocena "Ocena" sequential collection="[\"a\", \"b\", \"c\"]" item=r until="numberOfCompletedInstances >= 2" duration="1h" by=recenzenci
user rown "Recenzje równoległe" parallel collection="recenzje" item=r by=recenzenci duration="tri(1h, 2h, 6h)" assign="ok=bern(0.5)" output=oceny output-item="ok"
task pusta "Pusta lista" parallel collection="[]" output=nic output-item="1"
task nielista "Nie lista" parallel collection="liczba" duration="5m"
task blad "Błędna kolekcja" sequential collection="x ** 2" duration="5m"
task wyjscie "Błędne wyjście" parallel collection="[1, 2]" item=v output-item="v ** 2" output=bledy
task warunek "Błędny warunek" sequential collection="[1, 2, 3]" until="v ** 2" item=v
adhoc op "Opracowanie" active="[\"inw\", 7, \"brak\"]"
  task inw "Inwentarz" duration="2h"
  task sk "Skany" duration="5h"
adhoc wszystko "Wszystko naraz"
  task w1 "Pierwszy" duration="1h"
  task w2 "Drugi" duration="3h"
  task w3 "Po pierwszym" duration="30m"
  flow w1 -> w3
adhoc dosc "Do skutku" until="licznik >= 0"
  task d1 "Krok 1" duration="1h"
  task d2 "Krok 2" duration="4h"
adhoc czeka "Z oczekiwaniem" wait until="licznik >= 0"
  task c1 "Krok A" duration="1h"
  task c2 "Krok B" duration="2h"
adhoc nic "Nic do zrobienia" active="[]"
  task n1 "Nieużywany"
task rez "Rezerwacja" duration="5m" compensate=zw
task zw "Zwolnienie" duration="7m" compensation
task rez2 "Druga rezerwacja" duration="5m" compensate=zw2
task zw2 "Zwolnienie drugiej" duration="3m" compensation
throw cof "Cofnięcie" compensate activity=rez
throw cof2 "Cofnięcie powtórne" compensate activity=rez
end e "Koniec" compensate

flow s -> podw -> ocena -> rown -> pusta -> nielista -> blad -> wyjscie -> warunek -> op -> wszystko -> dosc -> czeka -> nic -> rez -> rez2 -> cof -> cof2 -> e

simulate cases=6 seed=1 arrival="exp(3h)"
resource recenzenci "Recenzenci" count=2
variable recenzje "cat(1=0.5, 2=0.5)"
variable liczba "unif(0, 1)"
variable licznik "unif(0, 1)"
''',
    'sim-err-cases.tok': r'''tok 0.1 "Błędna liczba spraw"

start s
task a "Praca" duration="1h"
end e
flow s -> a -> e

simulate cases=abc
''',
    'sim-err-cases-inf.tok': r'''tok 0.1 "Nieskończona liczba spraw"

start s
task a "Praca" duration="1h"
end e
flow s -> a -> e

simulate cases=inf
''',
    'sim-err-seed.tok': r'''tok 0.1 "Ziarno NaN"

start s
task a "Praca" duration="1h"
end e
flow s -> a -> e

simulate seed=nan
''',
    'sim-seed-float.tok': r'''tok 0.1 "Ziarno ułamkowe"

start s
task a "Praca" duration="1h"
end e
flow s -> a -> e

simulate seed=-7.9 cases=1e1
''',
    'sim-seed-big.tok': r'''tok 0.1 "Duże ziarno"

start s
task a "Praca" duration="1h"
end e
flow s -> a -> e

simulate seed=1e30 cases=3
''',
    'sim-err-start.tok': r'''tok 0.1 "Błędny start"

start s
task a "Praca" duration="1h"
end e
flow s -> a -> e

simulate start="2027-13-01"
''',
    'sim-err-start2.tok': r'''tok 0.1 "Niepoprawny start"

start s
task a "Praca" duration="1h"
end e
flow s -> a -> e

simulate start="jutro"
''',
    'sim-err-count.tok': r'''tok 0.1 "Błędna liczba wykonawców"

start s
task a "Praca" duration="1h"
end e
flow s -> a -> e

simulate
resource r count=x
''',
    'sim-err-dist.tok': r'''tok 0.1 "Błędny rozkład"

start s
task a "Praca" duration="1h"
end e
flow s -> a -> e

simulate arrival="exp(0)"
''',
    'sim-cases-zero.tok': r'''tok 0.1 "Zero spraw"

start s
task a "Praca" duration="1h"
end e
flow s -> a -> e

simulate cases=0
''',
    'sim-cases-neg.tok': r'''tok 0.1 "Ujemna liczba spraw"

start s
task a "Praca" duration="1h"
end e
flow s -> a -> e

simulate cases=-3.5
''',
    'sim-start-week.tok': r'''tok 0.1 "Start tygodniowy"

start s
task a "Praca" duration="1h"
end e
flow s -> a -> e

simulate start="2027W031" cases=2
''',
    'sim-start-aware.tok': r'''tok 0.1 "Start ze strefą"

start s
task a "Praca" duration="1h"
end e
flow s -> a -> e

simulate start="2027-01-04T08:00+01:00" cases=2
''',
    'sim-aware.tok': r'''tok 0.1 "Strefy czasowe"

start s
catch t "Termin" timer="2027-01-05T12:00"
end e

flow s -> t -> e

simulate start="2027-01-04T08:00Z" cases=2
''',
    'sim-aware-ok.tok': r'''tok 0.1 "Strefy czasowe zgodne"

start s
catch t "Termin" timer="2027-01-05T12:00+02:00"
task a "Praca" duration="1h"
boundary b "Limit" on=a timer="2027-01-04T08:30:00.5-01:00"
end e
end eb

flow s -> t -> a -> e
flow b -> eb

simulate start="2027-01-04T08:00Z" cases=3 arrival="30m"
''',
    'sim-cat-duration.tok': r'''tok 0.1 "Etykieta jako czas"

start s
task a "Praca" duration="cat(krótko=0.5, długo=0.5)"
end e

flow s -> a -> e

simulate cases=2
''',
    'sim-inf.tok': r'''tok 0.1 "Nieskończony czas"

start s
task a "Praca" duration="@BIG@"
end e

flow s -> a -> e

simulate cases=2
''',
    'sim-huge.tok': r'''tok 0.1 "Bardzo długi czas"

start s
task a "Praca" duration="100000000000000d"
end e

flow s -> a -> e

simulate cases=2
''',
    'sim-nostart.tok': r'''tok 0.1 "Bez startu"

task a "Praca"
end e
flow a -> e

simulate cases=2
''',
}

# seeds and case counts of the runs of every model with a simulate statement
SEEDS = [1, 7, 12345, -3]
CASES = [3, 20]


def write(path, text):
    with io.open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)


def read_text(path):
    with io.open(path, encoding="utf-8") as f:
        return f.read()


def last_traceback_line(ex):
    return traceback.format_exception_only(type(ex), ex)[-1].rstrip("\n")


def run_cli(args, env_lang=None):
    """Run the reference command line in this interpreter: (stdout, stderr, exit code)."""
    out, err = io.StringIO(), io.StringIO()
    saved = sys.stdout, sys.stderr
    if env_lang is None:
        os.environ.pop("TOK_LANG", None)
    else:
        os.environ["TOK_LANG"] = env_lang
    sys.stdout, sys.stderr = out, err
    try:
        code = cli.main(args)
    except SystemExit as ex:
        code = ex.code if isinstance(ex.code, int) else (0 if ex.code is None else 1)
    except Exception as ex:  # an exception the command line does not catch
        err.write(last_traceback_line(ex) + "\n")
        code = 1
    finally:
        sys.stdout, sys.stderr = saved
        os.environ.pop("TOK_LANG", None)
    return out.getvalue(), err.getvalue(), code


def documents(extra_dirs):
    """(source path, fixture name) of every input document."""
    found = []
    for p in sorted(glob.glob(os.path.join(TOK, "examples", "*.tok"))):
        found.append((p, os.path.basename(p)))
    for p in sorted(glob.glob(os.path.join(TOK, "conformance", "cases", "*.tok"))):
        found.append((p, "conformance-" + os.path.basename(p)))
    dirs = list(extra_dirs) or ([COURSE] if os.path.isdir(COURSE) else [])
    for d in dirs:
        for p in sorted(glob.glob(os.path.join(d, "*.tok"))):
            found.append((p, os.path.basename(p)))
    return found


def runs_of(name, text):
    """Argument lists (and TOK_LANG) for one document."""
    doc, diags = parse(text)
    has_sim = doc.simulate is not None
    out = []
    if not has_sim:
        out.append((["sim", name, "--cases", "3"], None))
        out.append((["sim", name, "--cases", "2", "--seed", "5", "--json"], None))
        out.append((["sim", name, "--cases", "10", "--lang", "pl"], None))
        return out
    out.append((["sim", name], None))
    out.append((["sim", name, "--json", "--cases", "15"], None))
    out.append((["sim", name], "pl"))
    k = 0
    for seed in SEEDS:
        for cases in CASES:
            args = ["sim", name, "--seed", str(seed), "--cases", str(cases)]
            if k % 2:
                args.append("--json")
            if k % 3 == 1:
                args += ["--lang", "pl"]
            out.append((args, None))
            k += 1
    out.append((["sim", name, "--cases", "300", "--seed", "99"], None))
    return out


CSV_RUNS = [
    ["sim", "kwerenda-symulacja.tok", "--cases", "60", "--log", OUT + "/log.csv", "--cases-csv",
     OUT + "/cases.csv"],
    ["sim", "sim-queue.tok", "--log", OUT + "/log.csv", "--cases-csv", OUT + "/cases.csv", "--json"],
    ["sim", "sim-rules.tok", "--cases", "8", "--log", OUT + "/log.csv", "--cases-csv", OUT + "/cases.csv"],
    ["sim", "sim-multi.tok", "--log", OUT + "/log.csv"],
    ["sim", "sim-boundary.tok", "--seed", "4", "--log", OUT + "/log.csv", "--cases-csv", ""],
    ["sim", "sim-huge.tok", "--log", OUT + "/log.csv", "--cases-csv", OUT + "/cases.csv"],
    ["sim", "sim-dists.tok", "--cases-csv", OUT + "/cases.csv"],
    ["sim", "sim-loop.tok", "--log", "brak-folderu/log.csv"],
    ["sim", "sim-loop.tok", "--log", ""],
    ["sim", "sim-loop.tok", "--log", "", "--cases-csv", OUT + "/cases.csv"],
]

# argument handling that Python 3.11, 3.12 and 3.13 agree on
ARG_RUNS = [
    [], ["-h"], ["--help"], ["--ca", "5"], ["--cases", "x"], ["--cases"], ["--cases=-5"],
    ["--cases", "-5"], ["--seed", "1_0"], ["--seed", "١٢"], ["--json=1"], ["-hx"],
    ["-hh"], ["--he"], ["-x"], ["--l", "pl"], ["--cas", "4"], ["--cases", "3", "extra"],
    ["--cases", "3", "-x"], ["--lang"], ["--lang", "pl", "--lang", "en"], ["--log"],
    ["--seed", "3.5"], ["--seed="], ["--se", "4"], ["--j"], ["--cases", "1", "--seed"],
    ["--cases", "2", "--json", "--json"], ["--seed", "99999999999999999999999999999999"],
    ["--seed", "-0"], ["--cases", "+2", "--seed", " 8 "],
]


def main():
    if sys.version_info < (3, 12):
        sys.exit("gen_python_sim.py needs Python 3.12 or newer (sum() of floats)")
    os.makedirs(FIX, exist_ok=True)
    # the Rust command line prints usage lines unwrapped, as argparse does on a terminal
    # wider than those lines
    os.environ["COLUMNS"] = "200"
    for f in glob.glob(os.path.join(FIX, "*.tok")):
        os.remove(f)
    names = []
    for p, name in documents(sys.argv[1:]):
        shutil.copyfile(p, os.path.join(FIX, name))
        names.append(name)
    for name, text in SYNTHETIC.items():
        write(os.path.join(FIX, name), text.replace("@BIG@", "1" + "0" * 339 + "s"))
        names.append(name)
    cwd = os.getcwd()
    os.chdir(FIX)
    cases = []
    try:
        def add(args, env_lang=None):
            tmp = tempfile.mkdtemp()
            try:
                real = [a.replace(OUT, tmp) for a in args]
                out, err, code = run_cli(real, env_lang)
                files = {}
                for f in sorted(os.listdir(tmp)):
                    with open(os.path.join(tmp, f), encoding="utf-8", newline="") as fh:
                        files[f] = fh.read()
            finally:
                shutil.rmtree(tmp)
            case = {"args": args, "stdout": out, "stderr": err, "code": code}
            if env_lang:
                case["env_lang"] = env_lang
            if files:
                case["files"] = files
            cases.append(case)

        for name in names:
            for args, env_lang in runs_of(name, read_text(name)):
                add(args, env_lang)
        for args in CSV_RUNS:
            add(args)
        for extra in ARG_RUNS:
            add(["sim", "sim-loop.tok"] + extra if extra else ["sim"])
        add(["sim", "brak-pliku.tok"])
        add(["sim", "sim-loop.tok", "--cases", "2"], env_lang="pl")
    finally:
        os.chdir(cwd)
    path = os.path.join(FIX, "cases-%s.json" % PLATFORM)
    write(path, json.dumps({"python": sys.version.split()[0], "cases": cases},
                           ensure_ascii=False, indent=1) + "\n")
    print("documents: %d, sim runs: %d -> %s" % (len(names), len(cases), path))


if __name__ == "__main__":
    main()
