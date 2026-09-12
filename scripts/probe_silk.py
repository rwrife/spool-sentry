"""Probe: silk-layer item bboxes (usage: probe_silk.py <board> [substr])."""
import sys
import pcbnew

b = pcbnew.LoadBoard(sys.argv[1])
sub = sys.argv[2] if len(sys.argv) > 2 else ""
MM = 1e6
for fp in b.GetFootprints():
    ref = fp.GetReference()
    if sub and sub not in ref:
        continue
    for g in fp.GraphicalItems():
        if "Silk" not in str(g.GetLayerName()):
            continue
        bb = g.GetBoundingBox()
        print(f"{ref} seg ({bb.GetLeft()/MM:6.2f},{bb.GetTop()/MM:6.2f})-({bb.GetRight()/MM:6.2f},{bb.GetBottom()/MM:6.2f})")
    for fld in fp.GetFields():
        if fld.GetName() != "Reference" or not fld.IsVisible():
            continue
        bb = fld.GetBoundingBox()
        print(f"{ref} REF({fld.GetLayerName()}) ({bb.GetLeft()/MM:6.2f},{bb.GetTop()/MM:6.2f})-({bb.GetRight()/MM:6.2f},{bb.GetBottom()/MM:6.2f})")
for t in b.GetDrawings():
    if type(t).__name__ != "PCB_TEXT":
        continue
    txt = t.GetText()
    if sub and sub not in txt:
        continue
    bb = t.GetBoundingBox()
    print(f"TEXT {txt!r} ({bb.GetLeft()/MM:6.2f},{bb.GetTop()/MM:6.2f})-({bb.GetRight()/MM:6.2f},{bb.GetBottom()/MM:6.2f})")
