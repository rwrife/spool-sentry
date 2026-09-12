"""Probe: placed pad positions for one footprint (usage: probe_padpos.py <board> <ref>)."""
import sys
import pcbnew

b = pcbnew.LoadBoard(sys.argv[1])
ref = sys.argv[2]
MM = 1e6
for fp in b.GetFootprints():
    if fp.GetReference() != ref:
        continue
    for pad in fp.Pads():
        p = pad.GetPosition()
        print(f"{pad.GetPadName():6} ({p.x/MM:7.2f}, {p.y/MM:7.2f})  net={pad.GetNetname()}")
