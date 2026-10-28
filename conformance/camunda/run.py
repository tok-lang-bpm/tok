"""Executable conformance: deploy every case to a running Camunda 8 and drive it to its expected end.

Usage (Camunda 8 Run started locally, REST API at http://localhost:8080/v2):

    PYTHONPATH=python/src python conformance/camunda/run.py [CASE ...]

For every case the script exports the tok document with `tok export`, deploys all produced
files, starts an instance, performs the listed actions (complete user tasks, complete or fail
jobs, set variables) and checks that the instance completed at the expected end event and
that the expected variables hold the expected values. Only the Python standard library and
the tok package are used.

Decision cases (language 0.7, `DMN_CASES`) take a conformance case of `conformance/cases`,
export its DMN 1.5 files with the native `tok` program (the environment variable `TOK`, else
`tok` on the PATH, else the program built in `dist/` or `rust/target/release/`), deploy them,
and compare the evaluation of each listed decision by Camunda with `tok eval` and with the
expected value.
"""

import glob
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
import uuid

HERE = os.path.dirname(os.path.abspath(__file__))
API = os.environ.get("CAMUNDA_API", "http://localhost:8080/v2")

# case: (process id, start variables, actions, expected end event, expected variables)
# actions: ("job", type, variables) completes one job of the type; ("jobs", type, n, variables) completes n jobs;
#          ("error", type, code, message[, variables]) throws a BPMN error from a job; ("user", n, variables) completes n user tasks;
#          ("set", variables) sets variables on the process instance; ("check-job", type, variables, headers)
#          activates a job and checks its variables and headers before completing it with no variables.
CASES = {
    "multi-instance": ("mi", {"pozycje": [1, 2, 3], "recenzje": ["a", "b", "c"]},
                       [("user", 2, {})], "e", {"wyniki": [2, 4, 6]}),
    "adhoc": ("adhoc", {"wybrane": ["inwentarz"]}, [], "e", {"inwentarz": "gotowy", "skany": None}),
    "call-activity": ("glowny", {"zamowienie": "Z1"}, [], "e", {"przesylka": "P-Z1", "numerPrzesylki": None}),
    "mappings": ("mapowania", {"zgloszenie": {"id": 7}, "priorytet": 80},
                 [("check-job", "rejestracja", {"numer": 7, "pilne": True}, {"szablon": "odmowa", "jezyk": "pl"},
                   {"wynik": {"sygnatura": "S-7"}})],
                 "e", {"sygnatura": "S-7"}),
    "user-task": ("zadanie", {}, [("check-user", {"assignee": "kierownik", "priority": 70}), ("user", 1, {})],
                  "e", {}),
    "message-job": ("wiadomosci", {}, [("job", "zawiadomienie", {}), ("job", "odpowiedz", {})], "e", {}),
    "compensation": ("kompensacja", {}, [("job", "rezerwacja", {}), ("job", "zwolnienie", {"zwolniono": True})],
                     "e", {"zwolniono": True}),
    "errors": ("bledy", {}, [("error", "praca", "E42", "brak danych", {"powod": "brak danych"})], "e2",
               {"powod": "brak danych"}),
    "conditional": ("warunek", {"kompletne": False}, [("set", {"kompletne": True})], "e", {}),
    "listeners": ("sluchacze", {}, [("job", "przed-wyliczeniem", {}), ("job", "utworzenie-zadania", {}),
                                    ("user", 1, {})], "e", {"wynik": 2}),
    "connector": ("lacznik", {}, [], "e", {"stan": "<any>"}),
    "version-tag": ("wersja", {}, [], "e", {}),
}
EXTRA_CHECKS = {"version-tag": ("versionTag", "v1")}

# decision cases of language 0.7: case of conformance/cases -> [(decision id, variables, expected value)]
DMN_CASES = {
    "61-executable-decisions": [
        ("limit", {"wiek": 16}, 5),
        ("limit", {"wiek": 30}, 10),
        ("blokada", {"wiek": 30, "Zaległe opłaty": 60}, True),
        ("blokada", {"wiek": 30, "Zaległe opłaty": 10}, False),
        ("naleznosc", {"dni-zwloki": 4, "opłata dzienna": 0.5}, 2),
        ("do-naprawy", {"egzemplarze": [{"sygnatura": "A1", "stan zachowania": "zły"},
                                        {"sygnatura": "B2", "stan zachowania": "dobry"}]}, ["A1"]),
        ("podsumowanie", {"wiek": 30, "Zaległe opłaty": 0, "dni-zwloki": 3, "opłata dzienna": 1},
         {"kwota": 4.5, "stan konta": "limit 10", "po zwłoce": True}),
        ("podsumowanie", {"wiek": 30, "Zaległe opłaty": 80, "dni-zwloki": 0, "opłata dzienna": 1},
         {"kwota": 0, "stan konta": "blokada", "po zwłoce": False}),
        ("kanaly", {}, ["e-mail", "SMS"]),
    ],
}
REPO = os.path.normpath(os.path.join(HERE, "..", ".."))


def call(method, path, body=None, files=None):
    url = API + path
    if files is not None:
        boundary = uuid.uuid4().hex
        parts = []
        for path_ in files:
            with open(path_, "rb") as f:
                data = f.read()
            parts.append(("--%s\r\nContent-Disposition: form-data; name=\"resources\"; filename=\"%s\"\r\n"
                          "Content-Type: application/octet-stream\r\n\r\n" % (boundary, os.path.basename(path_))
                          ).encode("utf-8") + data + b"\r\n")
        payload = b"".join(parts) + ("--%s--\r\n" % boundary).encode("utf-8")
        req = urllib.request.Request(url, data=payload, method=method,
                                     headers={"Content-Type": "multipart/form-data; boundary=" + boundary})
    else:
        data = json.dumps(body).encode("utf-8") if body is not None else None
        req = urllib.request.Request(url, data=data, method=method, headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=60) as r:
            text = r.read().decode("utf-8")
            return json.loads(text) if text.strip() else {}
    except urllib.error.HTTPError as ex:
        raise RuntimeError("%s %s -> %s %s" % (method, path, ex.code, ex.read().decode("utf-8", "replace")[:400]))


def poll(fn, what, timeout=60):
    t0 = time.time()
    while True:
        res = fn()
        if res:
            return res
        if time.time() - t0 > timeout:
            raise RuntimeError("timeout waiting for " + what)
        time.sleep(1)


def activate(job_type, n=1):
    def get():
        res = call("POST", "/jobs/activation", {"type": job_type, "timeout": 60000, "maxJobsToActivate": n,
                                                 "worker": "tok-conformance", "requestTimeout": 2000})
        return res.get("jobs") or None
    return poll(get, "job of type %s" % job_type)


def user_tasks(pik):
    res = call("POST", "/user-tasks/search", {"filter": {"processInstanceKey": pik, "state": "CREATED"}})
    return res.get("items") or None


def variables(pik):
    res = call("POST", "/variables/search", {"filter": {"processInstanceKey": pik}, "page": {"limit": 1000}})
    out = {}
    for v in res.get("items", []):
        if v.get("scopeKey") == pik or v.get("scopeKey") == str(pik):
            try:
                out[v["name"]] = json.loads(v["value"])
            except (ValueError, TypeError):
                out[v["name"]] = v["value"]
    return out


def export(name, folder):
    from tok.parser import parse
    from tok.check import check
    from tok.io import write_bpmn
    from tok import dmn, forms
    path = os.path.join(HERE, name + ".tok")
    with open(path, encoding="utf-8") as f:
        doc, diags = parse(f.read())
    diags += check(doc)
    errors = [d for d in diags if d.severity == "error"]
    if errors:
        raise RuntimeError("%s has errors: %s" % (name, ", ".join(d.code for d in errors)))
    files = [os.path.join(folder, name + ".bpmn")]
    write_bpmn(doc, files[0])
    for dec in doc.decisions.values():
        p = os.path.join(folder, dec.id + ".dmn")
        with open(p, "w", encoding="utf-8") as f:
            f.write(dmn.export_dmn(dec, doc))
        files.append(p)
    for form in doc.forms.values():
        p = os.path.join(folder, form.id + ".form")
        with open(p, "w", encoding="utf-8") as f:
            f.write(forms.export_form(form))
        files.append(p)
    return files


def run_case(name):
    pid, start_vars, actions, end_id, expected = CASES[name]
    folder = tempfile.mkdtemp(prefix="tok-camunda-")
    try:
        files = export(name, folder)
        call("POST", "/deployments", files=files)
        inst = call("POST", "/process-instances", {"processDefinitionId": pid, "variables": start_vars})
        pik = inst["processInstanceKey"]
        for act in actions:
            kind = act[0]
            if kind == "job":
                job = activate(act[1])[0]
                call("POST", "/jobs/%s/completion" % job["jobKey"], {"variables": act[2]})
            elif kind == "check-job":
                job = activate(act[1])[0]
                jv, jh = job.get("variables", {}), job.get("customHeaders", {})
                for k, v in act[2].items():
                    if jv.get(k) != v:
                        raise RuntimeError("job variable %s = %r, expected %r" % (k, jv.get(k), v))
                for k, v in act[3].items():
                    if jh.get(k) != v:
                        raise RuntimeError("job header %s = %r, expected %r" % (k, jh.get(k), v))
                if job.get("retries") is not None and name == "mappings" and job["retries"] != 5:
                    raise RuntimeError("job retries %r, expected 5" % job["retries"])
                call("POST", "/jobs/%s/completion" % job["jobKey"], {"variables": act[4]})
            elif kind == "error":
                job = activate(act[1])[0]
                body = {"errorCode": act[2], "errorMessage": act[3]}
                if len(act) > 4:
                    body["variables"] = act[4]
                call("POST", "/jobs/%s/error" % job["jobKey"], body)
            elif kind == "user":
                for _ in range(act[1]):
                    task = poll(lambda: user_tasks(pik), "user task")[0]
                    call("POST", "/user-tasks/%s/completion" % task["userTaskKey"], {"variables": act[2]})
                    time.sleep(1)
            elif kind == "check-user":
                task = poll(lambda: user_tasks(pik), "user task")[0]
                for k, v in act[1].items():
                    if task.get(k) != v:
                        raise RuntimeError("user task %s = %r, expected %r" % (k, task.get(k), v))
                for k in ("dueDate", "followUpDate", "candidateGroups"):
                    if not task.get(k):
                        raise RuntimeError("user task has no %s" % k)
            elif kind == "set":
                call("PUT", "/element-instances/%s/variables" % pik, {"variables": act[1]})

        def finished():
            res = call("POST", "/process-instances/search", {"filter": {"processInstanceKey": pik}})
            items = res.get("items") or []
            return items[0] if items and items[0].get("state") == "COMPLETED" else None
        poll(finished, "instance completion")

        def ended():
            res = call("POST", "/element-instances/search", {"filter": {"processInstanceKey": pik}})
            ids = [i["elementId"] for i in res.get("items", []) if i.get("state") == "COMPLETED"]
            return ids if end_id in ids else None
        poll(ended, "end event %s" % end_id)
        got = poll(lambda: variables(pik) or ({} if not expected else None), "variables") if expected else {}
        for k, v in expected.items():
            if v is None:
                if k in got:
                    raise RuntimeError("variable %s should not exist, got %r" % (k, got[k]))
            elif v == "<any>":
                if k not in got:
                    raise RuntimeError("variable %s missing" % k)
            elif got.get(k) != v:
                raise RuntimeError("variable %s = %r, expected %r" % (k, got.get(k), v))
        if name in EXTRA_CHECKS:
            key, val = EXTRA_CHECKS[name]
            res = call("POST", "/process-definitions/search", {"filter": {"processDefinitionId": pid}})
            items = res.get("items") or []
            if not items or items[-1].get(key) != val:
                raise RuntimeError("process definition %s = %r, expected %r" % (
                    key, items[-1].get(key) if items else None, val))
        return None
    except Exception as ex:  # report and continue with the next case
        return str(ex)
    finally:
        shutil.rmtree(folder, ignore_errors=True)


def tok_program():
    """The native `tok` program of the Rust engine."""
    if os.environ.get("TOK"):
        return os.environ["TOK"]
    found = shutil.which("tok")
    if found:
        return found
    for rel in ("dist", os.path.join("rust", "target", "release")):
        for exe in ("tok.exe", "tok"):
            p = os.path.join(REPO, rel, exe)
            if os.path.isfile(p):
                return p
    raise RuntimeError("no tok program: set TOK or build rust/tok-cli")


def run_dmn_case(name):
    tok = tok_program()
    case = os.path.join(REPO, "conformance", "cases", name + ".tok")
    folder = tempfile.mkdtemp(prefix="tok-camunda-")
    try:
        res = subprocess.run([tok, "export", case, "-o", folder], capture_output=True, text=True, encoding="utf-8")
        if res.returncode != 0:
            raise RuntimeError("tok export: " + (res.stderr or res.stdout).strip()[:400])
        call("POST", "/deployments", files=sorted(glob.glob(os.path.join(folder, "*.dmn"))))
        for decision, variables, expected in DMN_CASES[name]:
            args = [tok, "eval", case, decision] + [
                "%s=%s" % (k, v if isinstance(v, str) else json.dumps(v, ensure_ascii=False))
                for k, v in variables.items()]
            res = subprocess.run(args, capture_output=True, text=True, encoding="utf-8")
            if res.returncode != 0:
                raise RuntimeError("tok eval %s: %s" % (decision, res.stderr.strip()[:400]))
            mine = json.loads(res.stdout)
            out = call("POST", "/decision-definitions/evaluation",
                       {"decisionDefinitionId": decision, "variables": variables})
            theirs = json.loads(out["output"]) if isinstance(out.get("output"), str) else out.get("output")
            if mine != expected:
                raise RuntimeError("tok eval %s %r = %r, expected %r" % (decision, variables, mine, expected))
            if theirs != expected:
                raise RuntimeError("Camunda %s %r = %r, expected %r" % (decision, variables, theirs, expected))
        return None
    except Exception as ex:  # report and continue with the next case
        return str(ex)
    finally:
        shutil.rmtree(folder, ignore_errors=True)


def main(argv):
    names = argv or list(CASES) + list(DMN_CASES)
    failed = 0
    for name in names:
        err = run_dmn_case(name) if name in DMN_CASES else run_case(name)
        print("%-24s %s" % (name, "OK" if err is None else "FAIL: " + err))
        failed += err is not None
    print("%d of %d cases passed" % (len(names) - failed, len(names)))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
