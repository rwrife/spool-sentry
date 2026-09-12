"""Probe zones: layer, net, rule-area flag, priority, filled area (usage: probe_zones.py <board>)."""
import sys
import pcbnew

b = pcbnew.LoadBoard(sys.argv[1])
for i, z in enumerate(b.Zones()):
    print(f"zone{i} layer={z.GetLayerName()} net={z.GetNetname()!r} rule_area={z.GetIsRuleArea()} "
          f"prio={z.GetAssignedPriority()} filled_mm2={z.GetFilledArea()/1e12:.1f}")
errs = []
import json, subprocess
