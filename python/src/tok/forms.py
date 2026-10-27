"""Forms: export to Camunda form JSON and checks (spec §10.2)."""

import json
import re

FIELD_TYPES = ("textfield", "textarea", "number", "checkbox", "select", "radio", "datetime")
_NUMBER = re.compile(r"^-?[0-9]+(\.[0-9]+)?$")


def convert(text):
    """Option or default value: 'true'/'false' -> bool, numbers -> int/float, else the text."""
    t = text.strip()
    if t == "true":
        return True
    if t == "false":
        return False
    if _NUMBER.match(t):
        return float(t) if "." in t else int(t)
    return t


def parse_options(text):
    """'value:Label, value:Label' -> [{'label': ..., 'value': ...}]. Labels cannot contain commas."""
    out = []
    for part in text.split(","):
        part = part.strip()
        if not part:
            continue
        if ":" in part:
            value, label = part.split(":", 1)
            out.append({"label": label.strip(), "value": convert(value)})
        else:
            out.append({"label": part, "value": convert(part)})
    return out


def form_dict(form):
    """The Camunda form as a Python dict."""
    components = []
    ntext = 0
    for item in form.items:
        if item.kind == "text":
            ntext += 1
            components.append({"type": "text", "id": "Text_%d" % ntext, "text": item.label or ""})
            continue
        key = item.id
        typ = item.attrs.get("type", "textfield")
        c = {"type": typ, "id": "Field_%s" % key, "key": key}
        if typ == "datetime":
            c["subtype"] = "date"
            c["dateLabel"] = item.label or key
        else:
            c["label"] = item.label or key
        if typ in ("select", "radio") and "options" in item.attrs:
            c["values"] = parse_options(item.attrs["options"])
        if "default" in item.attrs:
            c["defaultValue"] = convert(item.attrs["default"])
        validate = {}
        if "required" in item.flags:
            validate["required"] = True
        if typ == "number":
            for k in ("min", "max"):
                if k in item.attrs:
                    validate[k] = convert(str(item.attrs[k]))
        if validate:
            c["validate"] = validate
        components.append(c)
    return {
        "id": form.id,
        "type": "default",
        "executionPlatform": "Camunda Cloud",
        "executionPlatformVersion": "8.10.0",
        "exporter": {"name": "tok", "version": "0.1.0"},
        "components": components,
        "schemaVersion": 18,
    }


def export_form(form):
    """Camunda form JSON text (file name: <form id>.form)."""
    return json.dumps(form_dict(form), ensure_ascii=False, indent=2) + "\n"


def check_form(form):
    """Return a list of (code, line, args) diagnostics for one form."""
    out = []
    for item in form.items:
        if item.kind != "field":
            continue
        typ = item.attrs.get("type", "textfield")
        if typ not in FIELD_TYPES:
            out.append(("M001", item.line, {"type": typ}))
        elif typ in ("select", "radio") and not parse_options(item.attrs.get("options", "")):
            out.append(("M002", item.line, {"id": item.id, "type": typ}))
    return out
