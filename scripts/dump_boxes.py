"""Placement hygiene dump for the generated carrier (usage: dump_boxes.py <board>).

Computes per-footprint courtyard ∪ pad bounding boxes in placed coordinates,
lists pairwise courtyard overlaps, board-edge violations, and drilled pads.
KiCad 9 bindings: footprint graphics come from GraphicalItems(); layer names
are plain strings (match substring "CrtYd").
"""
import sys
import pcbnew

b = pcbnew.LoadBoard(sys.argv[1] if len(sys.argv) > 1 else
                     'hardware/spool-sentry.kicad_pcb')
MM = 1e6

def crtyd_bbox(fp):
    pts = []
    for g in fp.GraphicalItems():
        lay = str(g.GetLayerName())
        if "CrtYd" not in lay:
            continue
        try:
            sh = g.Shape()
        except Exception:
            sh = None
        if sh == pcbnew.SHAPE_T_RECT:
            r = g.GetRect()
            pts += [(r.GetLeft(), r.GetTop()), (r.GetRight(), r.GetBottom())]
        elif sh == pcbnew.SHAPE_T_SEGMENT:
            pts += [(g.GetStart().x, g.GetStart().y), (g.GetEnd().x, g.GetEnd().y)]
        else:
            bb = g.GetBoundingBox()
            pts += [(bb.GetLeft(), bb.GetTop()), (bb.GetRight(), bb.GetBottom())]
    for pad in fp.Pads():
        p = pad.GetPosition()
        s = pad.GetSize()
        pts += [(p.x - s.x / 2, p.y - s.y / 2), (p.x + s.x / 2, p.y + s.y / 2)]
    if not pts:
        bb = fp.GetBoundingBox()
        return bb.GetLeft(), bb.GetTop(), bb.GetRight(), bb.GetBottom()
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    return min(xs), min(ys), max(xs), max(ys)

boxes = {}
for fp in b.GetFootprints():
    boxes[fp.GetReference()] = crtyd_bbox(fp)

items = sorted(boxes.items())
for ref, (x1, y1, x2, y2) in items:
    print(f"{ref:5} {x1/MM:7.2f},{y1/MM:7.2f} .. {x2/MM:7.2f},{y2/MM:7.2f}  "
          f"({(x2-x1)/MM:5.2f} x {(y2-y1)/MM:5.2f})")

print("--- courtyard/pad-box overlaps (>0.00 mm) ---")
for i in range(len(items)):
    for j in range(i + 1, len(items)):
        (r1, a), (r2, c) = items[i], items[j]
        ow = min(a[2], c[2]) - max(a[0], c[0])
        oh = min(a[3], c[3]) - max(a[1], c[1])
        if ow > 0 and oh > 0:
            print(f"  {r1} x {r2}: {ow/1e6:.2f} x {oh/1e6:.2f}")

print("--- board edge: box outside board ---")
W, H = 66.0 * MM, 75.0 * MM
for ref, (x1, y1, x2, y2) in items:
    out = []
    if x1 < 0: out.append(f"left {x1/MM:.2f}")
    if x2 > W: out.append(f"right +{(x2-W)/MM:.2f}")
    if y1 < 0: out.append(f"bottom {y1/MM:.2f}")
    if y2 > H: out.append(f"top +{(y2-H)/MM:.2f}")
    if out:
        print(f"  {ref}: {out}")
