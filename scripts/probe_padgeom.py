"""Probe: pad geometry of a footprint library file at a given rotation/position."""
import sys
import pcbnew

MM = 1e6
lib = sys.argv[1]
name = sys.argv[2]
rot = int(sys.argv[3]) if len(sys.argv) > 3 else 0

fp = pcbnew.FootprintLoad(f"/usr/share/kicad/footprints/{lib}.pretty", name)
if fp is None:
    sys.exit("not found")
fp.SetOrientationDegrees(rot)
# place at origin, compute pad bboxes in board coords
fp.SetPosition(pcbnew.VECTOR2I(0, 0))
for pad in fp.Pads():
    bb = pad.GetBoundingBox()
    print(f"{pad.GetPadName()!r:8} pos=({pad.GetPosition().x/MM:6.2f},{pad.GetPosition().y/MM:6.2f}) "
          f"box=({bb.GetLeft()/MM:6.2f},{bb.GetTop()/MM:6.2f})-({bb.GetRight()/MM:6.2f},{bb.GetBottom()/MM:6.2f}) "
          f"layers={pad.IsOnLayer(pcbnew.F_Cu)}/{pad.IsOnLayer(pcbnew.B_Cu)}")
# courtyard
pts=[]
for g in fp.Graphs():
    if "CrtYd" in str(g.LayerName()):
        bb=g.GetBoundingBox()
        pts.append((bb.GetLeft(),bb.GetTop(),bb.GetRight(),bb.GetBottom()))
if pts:
    print("crtyd", min(p[0] for p in pts)/MM, min(p[1] for p in pts)/MM, max(p[2] for p in pts)/MM, max(p[3] for p in pts)/MM)
