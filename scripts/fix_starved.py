"""Fix starved_thermal on mechanical/anchor pads (usage: fix_starved.py <board> <ref> [pads...]).

KiCad 9 renamed the setter: PAD.SetZoneConnection does not exist; use
SetLocalZoneConnection(pcbnew.ZONE_CONNECTION_FULL).
"""
import sys
import pcbnew

b = pcbnew.LoadBoard(sys.argv[1])
ref = sys.argv[2]
want = set(sys.argv[3:])
for fp in b.GetFootprints():
    if fp.GetReference() != ref:
        continue
    for pad in fp.Pads():
        if want and pad.GetPadName() not in want:
            continue
        pad.SetLocalZoneConnection(pcbnew.ZONE_CONNECTION_FULL)
        print("FULL", ref, pad.GetPadName())
b.Save(sys.argv[1])
print("saved")
