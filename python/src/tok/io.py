"""Reading tok files and writing their projections to disk."""

import os
import xml.etree.ElementTree as ET

from . import bpmnlib
from .bpmn_export import export_tree, ZEEBE, MODELER
from .parser import parse


def read(path):
    """Parse a tok file. Return (Document, diagnostics)."""
    with open(path, encoding="utf-8") as f:
        return parse(f.read())


def write_bpmn(doc, path, layout=True, target=None):
    """Write BPMN 2.0 XML with diagram layout (BPMNDI). Return the bpmnlib Model."""
    tree = export_tree(doc, target)
    for prefix, uri in bpmnlib.NS.items():
        ET.register_namespace(prefix, uri)
    m = bpmnlib.Model(tree)
    if layout:
        bpmnlib.layout(m)
    in_map = {"zeebe": ZEEBE, "modeler": MODELER} if doc.executable_any else {}
    bpmnlib.save(tree, path, in_map)
    return m


def write_svg(doc, path, target=None):
    tree = export_tree(doc, target)
    for prefix, uri in bpmnlib.NS.items():
        ET.register_namespace(prefix, uri)
    m = bpmnlib.Model(tree)
    bpmnlib.layout(m)
    with open(path, "w", encoding="utf-8") as f:
        f.write(bpmnlib.render_svg(m, title=os.path.basename(path)))
