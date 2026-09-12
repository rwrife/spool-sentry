"""Fill copper zones and report filled area per zone (usage: fill_zones.py <board>).

The GND pours connect the GND pads that the autorouter cannot see (zones are
not exported into Specctra DSN), so fill must happen between SES import and
the final DRC adjudication.
"""
import sys
import pcbnew

b = pcbnew.LoadBoard(sys.argv[1])
for z in b.Zones():
    print("zone", z.GetLayerName(), "net", repr(z.GetNetname()), "rule_area", z.GetIsRuleArea())
filler = pcbnew.ZONE_FILLER(b)
filler.Fill(b.Zones())
b.Save(sys.argv[1])
b2 = pcbnew.LoadBoard(sys.argv[1])
for z in b2.Zones():
    if z.GetIsRuleArea():
        continue
    print("filled", z.GetLayerName(), z.GetNetname(), f"{z.GetFilledArea()/1e12:.1f} mm2")
