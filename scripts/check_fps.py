import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from build_pcb import PLACEMENTS, MOUNT_HOLES, load_footprint

for ref, lib, name, *_ in PLACEMENTS:
    try:
        fp = load_footprint(lib, name)
        pads = sorted(p.GetPadName() for p in fp.Pads())
        print(f"OK   {ref:5} {lib}:{name} pads={pads}")
    except SystemExit as e:
        print(f"MISS {ref:5} {lib}:{name}")
for ref, x, y in MOUNT_HOLES:
    try:
        load_footprint("MountingHole", "MountingHole_3.2mm_M3")
        print("OK   MH")
        break
    except SystemExit:
        print("MISS MH")
