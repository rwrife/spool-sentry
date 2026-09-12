"""Sync footprint Value fields to the schematic values (usage: sync_values.py <board>).

Cross-analysis XV-002 flagged U1 value mismatch — the generator set the
footprint Value to the module short name while the schematic carries the
full manufacturer part description. Keep PCB values identical to schematic
properties (single-source-of-truth rule).
"""
import sys
import pcbnew

SYNC = {"U1": "Seeed XIAO ESP32-C3"}

b = pcbnew.LoadBoard(sys.argv[1])
for fp in b.GetFootprints():
    ref = fp.GetReference()
    if ref in SYNC:
        for fld in fp.GetFields():
            if fld.GetName() == "Value":
                print(f"{ref}: {fld.GetText()!r} -> {SYNC[ref]!r}")
                fld.SetText(SYNC[ref])
b.Save(sys.argv[1])
print("saved")
