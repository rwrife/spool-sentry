"""Probe: per-pad net names on a saved board (usage: probe_netbind.py <board> [refs...])."""
import sys
import pcbnew

board = pcbnew.LoadBoard(sys.argv[1])
refs = set(sys.argv[2:])
print("net count:", board.GetNetCount())
for fp in board.GetFootprints():
    ref = fp.GetReference()
    if refs and ref not in refs:
        continue
    nets = {}
    for pad in fp.Pads():
        nets[pad.GetPadName()] = pad.GetNetname()
    print(ref, nets)
