#!/usr/bin/env python3
"""Deterministic carrier-PCB generator for Spool Sentry (issue #3).

Builds hardware/spool-sentry.kicad_pcb from the approved issue-#2 netlist
using pcbnew (KiCad 9 python) so every element is KiCad-serialized.

This is a hand-placed v0.1 "A0" carrier:
  * 66.0 x 75.0 mm two-layer board, USB edge and load-cell cable exit on the
    left, debug header on the bottom edge.
  * XIAO ESP32-C3 top-side with an antenna-end keepout enforced as a KiCad
    rule area (copper-free on both copper layers).
  * B.Cu and F.Cu GND pours; analog bridge front-end grouped away from U1;
    environmental sensor in the far corner from MCU/module heat.
  * Four M3 mounting holes; short fan-out stubs on SMD pads for assembly.
  * Copper routing itself is produced by the autorouter pass documented in
    docs/verification/issue-3-pcb.md.

Run inside the parts-tally-kicad:9-arm64 image:
    python3 scripts/build_pcb.py [--out hardware/spool-sentry.kicad_pcb]
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pcbnew

MM = 1_000_000
HERE = Path(__file__).resolve().parents[1]

BOARD_W, BOARD_H = 66.0, 75.0

# ---------------------------------------------------------------- layout ---
# (ref, lib, fp, x_mm, y_mm, rot_deg, side)  side: 'F' top, 'B' bottom
PLACEMENTS = [
    # --- USB power chain (bottom side; J1 at top-left edge, pad rows face
    #     inward per pitfall #34; 2.6 mm inset keeps the -x shell tabs on-board).
    #     Row at y=61 spaced by courtyard widths (D1 4.4, F1 5.9, D2 7.0,
    #     R1/R2 2.96) with >=0.3 mm courtyard gaps.
    ("J1", "Connector_USB", "USB_C_Receptacle_GCT_USB4105-xx-A_16P_TopMnt_Horizontal", 2.6, 68.0, 270, "B"),
    ("D1", "Diode_SMD", "D_SOD-123F", 9.4, 61.0, 0, "B"),
    ("F1", "Fuse", "Fuse_1812_4532Metric", 14.95, 61.0, 180, "B"),
    ("D2", "Diode_SMD", "D_SMA", 21.75, 61.0, 180, "B"),
    ("C11", "Capacitor_SMD", "C_0603_1608Metric", 10.5, 64.5, 0, "B"),
    ("C1", "Capacitor_SMD", "C_1206_3216Metric", 15.0, 65.0, 0, "B"),
    ("C2", "Capacitor_SMD", "C_0603_1608Metric", 22.5, 65.0, 0, "B"),
    ("R1", "Resistor_SMD", "R_0603_1608Metric", 28.3, 61.0, 0, "B"),
    ("R2", "Resistor_SMD", "R_0603_1608Metric", 31.9, 61.0, 0, "B"),
    # --- controller (top side, antenna end toward the bottom-left corner)
    ("U1", "spool-sentry", "XIAO_ESP32C3", 23.5, 47.0, 0, "F"),
    # --- analog: ADC + bridge front-end (right/centre column, away from U1)
    ("U2", "Package_SO", "SOIC-16_3.9x9.9mm_P1.27mm", 36.5, 21.0, 0, "F"),
    ("R3", "Resistor_SMD", "R_0603_1608Metric", 30.5, 12.0, 0, "B"),
    ("R4", "Resistor_SMD", "R_0603_1608Metric", 30.5, 9.0, 0, "B"),
    ("C8", "Capacitor_SMD", "C_0603_1608Metric", 34.0, 10.5, 0, "B"),
    ("C6", "Capacitor_SMD", "C_0603_1608Metric", 32.0, 17.0, 90, "F"),
    ("C5", "Capacitor_SMD", "C_0603_1608Metric", 41.5, 16.5, 90, "F"),
    ("C4", "Capacitor_SMD", "C_0603_1608Metric", 41.5, 20.0, 90, "F"),
    ("C7", "Capacitor_SMD", "C_0603_1608Metric", 41.5, 24.0, 90, "F"),
    ("C9", "Capacitor_SMD", "C_0603_1608Metric", 41.5, 27.5, 90, "F"),
    ("J2", "Connector_JST", "JST_GH_SM04B-GHS-TB_1x04-1MP_P1.25mm_Horizontal", 3.3, 13.0, 270, "B"),
    # --- debug header (bottom edge)
    ("J3", "Connector_PinHeader_2.54mm", "PinHeader_1x04_P2.54mm_Vertical", 46.0, 3.5, 0, "F"),
    # --- environmental sensor: corner far from U1/module heat (vented zone)
    ("U3", "Sensor_Humidity", "Sensirion_DFN-4_1.5x1.5mm_P0.8mm_SHT4x_NoCentralPad", 62.0, 44.0, 90, "F"),
    # 5V bulk cap for the module, east of the antenna keepout (x>33) and of
    # U1's courtyard (x>32.5), beside the module's 5V pad row
    ("C3", "Capacitor_SMD", "C_1206_3216Metric", 36.0, 40.5, 0, "F"),
    ("R5", "Resistor_SMD", "R_0603_1608Metric", 35.5, 31.6, 0, "F"),
    ("R6", "Resistor_SMD", "R_0603_1608Metric", 39.0, 31.6, 0, "F"),
    # --- user interface (top edge; LED/LED-R row on 3.3 mm pitch, courtyard
    #     2.96 wide + 0.34 gap)
    ("SW1", "Button_Switch_SMD", "SW_SPST_PTS810", 46.0, 69.0, 0, "F"),
    ("R7", "Resistor_SMD", "R_0603_1608Metric", 51.5, 69.0, 0, "F"),
    ("C10", "Capacitor_SMD", "C_0603_1608Metric", 51.5, 71.5, 0, "F"),
    ("D3", "LED_SMD", "LED_Avago_PLCC6_3x2.8mm", 56.0, 68.0, 0, "F"),
    ("R8", "Resistor_SMD", "R_0603_1608Metric", 54.4, 64.5, 0, "F"),
    ("R9", "Resistor_SMD", "R_0603_1608Metric", 57.7, 64.5, 0, "F"),
    ("R10", "Resistor_SMD", "R_0603_1608Metric", 61.0, 64.5, 0, "F"),
    # --- test points (near their nets, top side, clear of courtyards/keepouts)
    ("TP1", "TestPoint", "TestPoint_Pad_D1.5mm", 12.0, 69.5, 0, "F"),
    ("TP2", "TestPoint", "TestPoint_Pad_D1.5mm", 40.0, 40.5, 0, "F"),
    ("TP3", "TestPoint", "TestPoint_Pad_D1.5mm", 40.0, 44.0, 0, "F"),
    ("TP4", "TestPoint", "TestPoint_Pad_D1.5mm", 40.0, 47.5, 0, "F"),
    ("TP5", "TestPoint", "TestPoint_Pad_D1.5mm", 27.5, 21.0, 0, "F"),
    ("TP6", "TestPoint", "TestPoint_Pad_D1.5mm", 6.0, 34.0, 0, "F"),
    ("TP7", "TestPoint", "TestPoint_Pad_D1.5mm", 8.5, 34.0, 0, "F"),
    ("TP8", "TestPoint", "TestPoint_Pad_D1.5mm", 41.0, 12.0, 0, "F"),
    ("TP9", "TestPoint", "TestPoint_Pad_D1.5mm", 41.0, 9.0, 0, "F"),
    ("TP10", "TestPoint", "TestPoint_Pad_D1.5mm", 30.0, 21.0, 0, "F"),
]

MOUNT_HOLES = [("MH1", 3.6, 3.6), ("MH2", 62.4, 3.6), ("MH3", 62.4, 71.4), ("MH4", 3.6, 48.0)]

# Schematic properties are the source of truth for footprint values (XV-002);
# the project footprint template carries only the short module name.
VALUE_OVERRIDES = {"U1": "Seeed XIAO ESP32-C3"}

# Net classes: (name, track_mm, clearance_mm, via_drill_mm, via_diam_mm, nets)
NET_CLASSES = [
    ("Power", 0.4, 0.25, 0.3, 0.7, ["VBUS", "VBUS_FUSED", "+5V_XIAO", "+3V3"]),
    ("Analog", 0.2, 0.2, 0.3, 0.6,
     ["AIN+", "AIN-", "LC_S+", "LC_S-", "AVDD_3V0", "VBG", "PGA_CFILTER"]),
    ("Default", 0.2, 0.2, 0.3, 0.6, []),
]

# Antenna-end rule area: the XIAO C3 PCB antenna sits at the pad-1 (=-Y) end
# of the module (antenna body footprint-local -4..3 x, -12.5..-8 y -> placed
# 19.5..26.5 x, 34.5..39.0 y). The keepout wraps that end with >=3 mm margin
# and is enforced copper-free on both layers. No part copper lies in it.
KEEPOUTS = [
    ("AntennaKeepout", 16.0, 25.0, 31.0, 40.5),
    ("EdgeConnectorKeepout", 0.0, 60.5, 8.5, 75.0),
    ("LoadCellEdgeKeepout", 0.0, 5.0, 10.5, 18.5),
]

SILK_TEXTS = [
    ("SPOOL SENTRY  CARRIER  REV A0", 34.0, 73.0, 1.0),
    ("USB 5V SELV ONLY - OBSERVATION ONLY - MIT", 25.5, 1.6, 0.8),
    ("USB 5V", 4.5, 57.5, 0.8),
    ("LOAD CELL", 8.0, 21.0, 0.8),
    ("UART 3V3 GND TX RX", 54.5, 6.0, 0.8),
    ("TARE", 46.0, 65.4, 0.8),
    ("GND", 43.5, 44.0, 0.8),
    ("3V3", 43.5, 47.5, 0.8),
    ("5V", 43.5, 40.5, 0.8),
    ("VBUS", 16.0, 69.8, 0.8),
    ("AVDD", 24.5, 22.0, 0.8),
    ("SDA", 6.0, 38.5, 0.8),
    ("SCL", 8.5, 38.5, 0.8),
    ("AIN+", 43.2, 14.2, 0.8),
    ("AIN-", 45.5, 17.0, 0.8),
    ("VBG", 24.0, 19.5, 0.8),
    ("ANTENNA KEEPOUT - NO COPPER - NO METAL", 23.5, 27.5, 0.8),
]


def load_footprint(lib, name):
    if lib == "spool-sentry":
        libdir = HERE / "hardware" / "lib" / "spool-sentry.pretty"
    else:
        libdir = Path("/usr/share/kicad/footprints") / (lib + ".pretty")
    fp = pcbnew.FootprintLoad(str(libdir), name)
    if fp is None:
        raise SystemExit(f"footprint not found: {lib}:{name} in {libdir}")
    return fp


def main():
    out = Path(sys.argv[sys.argv.index("--out") + 1]) if "--out" in sys.argv else \
        HERE / "hardware" / "spool-sentry.kicad_pcb"
    pin_net = json.loads((HERE / ".build-pcb-pinnet.json").read_text())

    board = pcbnew.NewBoard(str(out))

    # Register nets, then re-fetch the board-owned objects and keep them
    # referenced until Save: python-side NETINFO_ITEM proxies are otherwise
    # garbage-collected and the nets vanish from the saved board.
    netmap = {}
    for n in sorted(set(pin_net.values())):
        board.Add(pcbnew.NETINFO_ITEM(board, n))
        netmap[n] = board.FindNet(n)
        if netmap[n] is None:
            raise SystemExit(f"net registration failed: {n}")

    ns = board.GetDesignSettings().m_NetSettings
    for name, w, clr, drill, diam, nets in NET_CLASSES:
        if name == "Default":
            dnc = ns.GetDefaultNetclass()
            dnc.SetTrackWidth(int(w * MM))
            dnc.SetClearance(int(clr * MM))
            dnc.SetViaDrill(int(drill * MM))
            dnc.SetViaDiameter(int(diam * MM))
            continue
        nc = pcbnew.NETCLASS(name)
        nc.SetTrackWidth(int(w * MM))
        nc.SetClearance(int(clr * MM))
        nc.SetViaDrill(int(drill * MM))
        nc.SetViaDiameter(int(diam * MM))
        ns.SetNetclass(name, nc)
    for name, _w, _c, _d, _dm, nets in NET_CLASSES:
        if name == "Default":
            continue
        nc2 = ns.GetNetClassByName(name)
        for n in nets:
            netmap[n].SetNetClass(nc2)

    # board outline
    corners = [(0, 0), (BOARD_W, 0), (BOARD_W, BOARD_H), (0, BOARD_H)]
    for i in range(4):
        s = pcbnew.PCB_SHAPE(board)
        s.SetShape(pcbnew.SHAPE_T_SEGMENT)
        s.SetWidth(int(0.1 * MM))
        s.SetStart(pcbnew.VECTOR2I(int(corners[i][0] * MM), int(corners[i][1] * MM)))
        s.SetEnd(pcbnew.VECTOR2I(int(corners[(i + 1) % 4][0] * MM), int(corners[(i + 1) % 4][1] * MM)))
        s.SetLayer(pcbnew.Edge_Cuts)
        board.Add(s)

    # footprints + net assignment
    missing = []
    for ref, lib, fpname, x, y, rot, side in PLACEMENTS:
        fp = load_footprint(lib, fpname)
        fp.SetReference(ref)
        if ref in VALUE_OVERRIDES:
            for fld in fp.GetFields():
                if fld.GetName() == "Value":
                    fld.SetText(VALUE_OVERRIDES[ref])
        fp.SetLayer(pcbnew.F_Cu if side == "F" else pcbnew.B_Cu)
        fp.SetPosition(pcbnew.VECTOR2I(int(x * MM), int(y * MM)))
        fp.SetOrientationDegrees(rot)
        assigned = set()
        for pad in fp.Pads():
            key = f"{ref}.{pad.GetPadName()}"
            net = pin_net.get(key)
            if net is None and ref == "J2" and pad.GetPadName() == "MP":
                # JST GH shield/anchor pads bond to the GND pour per issue #3
                # grounding decision (documented in issue-3-pcb.md).
                net = "GND"
            if net is None and ref == "J1" and pad.GetPadName() == "":
                # unnamed USB-C shell mounting tabs; BOM: "Shell to GND".
                net = "GND"
            if net:
                pad.SetNet(netmap[net])
                assigned.add(pad.GetPadName())
        # report pads without a net (NC / unused / naming mismatch)
        for pad in fp.Pads():
            if pad.GetPadName() not in assigned:
                missing.append(f"{ref}.{pad.GetPadName()}")
        board.Add(fp)

    print("pads with no net (expected NC/unused):")
    for m in missing:
        print("  ", m)

    # mounting holes
    for ref, x, y in MOUNT_HOLES:
        fp = load_footprint("MountingHole", "MountingHole_3.2mm_M3")
        fp.SetReference(ref)
        fp.SetLayer(pcbnew.F_Cu)
        fp.SetPosition(pcbnew.VECTOR2I(int(x * MM), int(y * MM)))
        board.Add(fp)

    # GND pours (B.Cu primary ground, F.Cu secondary)
    pour = [(0.4, 0.4), (BOARD_W - 0.4, 0.4), (BOARD_W - 0.4, BOARD_H - 0.4), (0.4, BOARD_H - 0.4)]
    gnd = netmap["GND"]
    for layer in (pcbnew.B_Cu, pcbnew.F_Cu):
        z = pcbnew.ZONE(board)
        lc = pcbnew.SHAPE_LINE_CHAIN()
        for px, py in pour:
            lc.Append(int(px * MM), int(py * MM))
        lc.SetClosed(True)
        z.AddPolygon(lc)
        z.SetNetCode(gnd.GetNetCode())
        z.SetLayer(layer)
        z.SetPadConnection(pcbnew.ZONE_CONNECTION_THERMAL)
        board.Add(z)

    # antenna / edge keepout rule areas on both copper layers.
    # Pitfall #33: default rule areas also forbid pads/footprints, which would
    # flag the very parts living in the zone (edge connector, load-cell
    # connector, mounting holes). These zones exist only to carve copper.
    for name, x1, y1, x2, y2 in KEEPOUTS:
        for layer in (pcbnew.F_Cu, pcbnew.B_Cu):
            z = pcbnew.ZONE(board)
            lc = pcbnew.SHAPE_LINE_CHAIN()
            for px, py in [(x1, y1), (x2, y1), (x2, y2), (x1, y2)]:
                lc.Append(int(px * MM), int(py * MM))
            lc.SetClosed(True)
            z.AddPolygon(lc)
            z.SetIsRuleArea(True)
            z.SetDoNotAllowPads(False)
            z.SetDoNotAllowTracks(False)
            z.SetDoNotAllowVias(False)
            z.SetDoNotAllowFootprints(False)
            z.SetDoNotAllowCopperPour(True)
            z.SetAssignedPriority(1)  # carve the priority-0 GND pours (pitfall #25)
            z.SetLayer(layer)
            board.Add(z)

    # documentation silkscreen. Pitfall #21: SetTextWidth alone does not
    # satisfy the text_thickness rule for gr_text — set the explicit stroke
    # thickness (and do not call SetTextWidth, which corrupts the font box).
    for text, x, y, size in SILK_TEXTS:
        t = pcbnew.PCB_TEXT(board)
        t.SetText(text)
        t.SetPosition(pcbnew.VECTOR2I(int(x * MM), int(y * MM)))
        t.SetLayer(pcbnew.F_SilkS)
        t.SetVisible(True)
        t.SetTextSize(pcbnew.VECTOR2I(int(size * MM), int(size * MM)))
        t.SetTextThickness(int(0.15 * MM))
        board.Add(t)

    # Reference-field hygiene (pitfall #26): hiding references is fine for
    # dense passives, WRONG for user-facing/test parts — keep those visible at
    # DRC-legal size (>= 1.0 mm height, >= 0.15 mm stroke) and offset them off
    # their own pads/nets so the silk stays readable.
    TP_REF_REL = {  # (dx, dy) mm from pad center; keeps refs off own circle,
        # neighbouring circles/pads, and the net labels (anchors are centred)
        "TP1": (0.0, 2.3), "TP2": (0.0, -2.5), "TP3": (-2.7, 0.0),
        "TP4": (0.0, 2.3), "TP5": (0.0, -2.3), "TP6": (-0.9, 2.3),
        "TP7": (-0.9, -2.3), "TP8": (-0.9, 2.3), "TP9": (-0.9, -2.3),
        "TP10": (0.0, 2.3), "SW1": (-3.0, 2.6), "J1": (0.0, 0.0),
        "J2": (0.0, 0.0), "D3": (0.0, 2.6), "J3": (3.0, 0.0),
        "D1": (0.0, 1.9), "D2": (0.0, -3.0),
    }
    for fp in board.GetFootprints():
        ref = fp.GetReference()
        # Bottom-side parts: SetLayer() mirrors pads but leaves graphics and
        # fields on the F layers; move them to the matching B layers AND
        # mirror the text, or every back-layer glyph flags
        # nonmirrored_text_on_back_layer.
        if fp.GetLayer() == pcbnew.B_Cu:
            move = {}
            for a, bnm in (("F_SilkS", "B_SilkS"), ("F_CrtYd", "B_CrtYd"),
                           ("F_Fab", "B_Fab"), ("F_Paste", "B_Paste"),
                           ("F_Mask", "B_Mask")):
                la, lb = getattr(pcbnew, a, None), getattr(pcbnew, bnm, None)
                if la is not None and lb is not None:
                    move[la] = lb
            for g in fp.GraphicalItems():
                g.SetLayer(move.get(g.GetLayer(), g.GetLayer()))
                if hasattr(g, "SetMirrored"):
                    g.SetMirrored(True)
            for fld in fp.GetFields():
                if fld.GetLayer() in move:
                    fld.SetLayer(move[fld.GetLayer()])
                fld.SetMirrored(True)
        for fld in fp.GetFields():
            if fld.GetName() != "Reference":
                continue
            if ref[0] in "RCF" or ref.startswith("MH"):
                fld.SetVisible(False)
            elif ref in TP_REF_REL:
                fld.SetVisible(True)
                fld.SetTextSize(pcbnew.VECTOR2I(int(1.0 * MM), int(1.0 * MM)))
                fld.SetTextThickness(int(0.15 * MM))
                pos = fp.GetPosition()
                dx, dy = TP_REF_REL[ref]
                fld.SetTextPos(pcbnew.VECTOR2I(pos.x + int(dx * MM),
                                               pos.y + int(dy * MM)))
                if fp.GetLayer() == pcbnew.B_Cu:
                    fld.SetMirrored(True)

    board.BuildConnectivity()
    out.parent.mkdir(parents=True, exist_ok=True)
    board.Save(str(out))
    print(f"saved {out}")


if __name__ == "__main__":
    main()
