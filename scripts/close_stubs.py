"""Deterministically close short unrouted stubs (usage: close_stubs.py <board> <drc.json>).

Pitfall #20 recipe: parse DRC unconnected_items pairs (pad<->pad, pad<->track
end, track-end<->track-end), build the candidate segment as a manually
buffered capsule polygon (segment expanded by width/2 + clearance), and
accept only when it collides with no foreign same-layer pad or track
(pitfall #31: SHAPE_POLY_SET has no BufferOutline; Collide(shape, 0) is the
pair-distance test; PAD.GetNetCode not GetNetcode; pads unhashable -> key by
id()). Zone/shell pseudo-ratsnest pairs are skipped (bonded by the filled
GND pour and documented). Adjudicate with a fresh DRC; the script saves only
the accepted set.
"""
import json, math, re, sys
import pcbnew

MM = 1_000_000
CLEARANCE = 0.2
POWER_NETS = {"VBUS", "VBUS_FUSED", "+5V_XIAO", "+3V3"}
OUT_MARGIN = 0.5  # mm, copper edge clearance guard


def net_of(desc):
    m = re.search(r"\[/?(.*?)\]", desc)
    return m.group(1) if m else None


def layer_of(desc):
    if "on F.Cu" in desc:
        return pcbnew.F_Cu
    if "on B.Cu" in desc:
        return pcbnew.B_Cu
    return None


def parse_side(desc):
    """Return ('pad', ref, padname, layer_or_None, net) or ('track', net, length_mm, layer)."""
    net = net_of(desc)
    m = re.match(r"(?:SMD pad|PTH pad|Pad)\s+(\S+)\s+\[[^\]]*\] of (\S+)", desc)
    if m:
        return ("pad", m.group(2), m.group(1), layer_of(desc), net)
    if desc.startswith("Track "):
        m = re.search(r"length ([0-9.]+) mm", desc)
        return ("track", net, float(m.group(1)) if m else None, layer_of(desc))
    return None


def capsule(x1, y1, x2, y2, hw):
    dx, dy = x2 - x1, y2 - y1
    ln = math.hypot(dx, dy) or 1.0
    nx, ny = -dy / ln * hw, dx / ln * hw
    chain = pcbnew.SHAPE_LINE_CHAIN()
    for px, py in [(x1 + nx, y1 + ny), (x2 + nx, y2 + ny), (x2 - nx, y2 - ny), (x1 - nx, y1 - ny)]:
        chain.Append(int(px), int(py))
    chain.SetClosed(True)
    return pcbnew.SHAPE_POLY_SET(chain)


def main():
    board_path, drc_path = sys.argv[1], sys.argv[2]
    board = pcbnew.LoadBoard(board_path)
    pads = {}
    for fp in board.GetFootprints():
        for pad in fp.Pads():
            pads[(fp.GetReference(), pad.GetPadName())] = pad
    tracks = [t for t in board.GetTracks() if t.Type() == pcbnew.PCB_TRACE_T]
    track_pool = list(tracks)  # consumed on use to disambiguate same-length tracks

    drc = json.load(open(drc_path))
    accepted, skipped = [], []

    # net code -> name map. KiCad 9 python has no BOARD.GetNetsById/BOARD.Nets;
    # derive the mapping from pad net attributes (covers every named net).
    code2name = {0: ""}
    for fp in board.GetFootprints():
        for pad in fp.Pads():
            code2name[pad.GetNetCode()] = pad.GetNetname()

    # fill any missing names from GetNetsByName
    try:
        for name, ni in board.GetNetsByName().items():
            code2name[ni.GetNetCode()] = str(name)
    except Exception:
        pass

    def netname(code):
        return code2name.get(code, "")

    for u in drc.get("unconnected_items", []):
        descs = [it.get("description", "") for it in u.get("items", [])]
        joined = " ".join(descs)
        if "Zone" in joined or re.search(r"(S1|MP) \[?/?GND", joined) or "PTH pad S1" in joined:
            skipped.append(("pseudo", joined[:90]))
            continue
        sides = [parse_side(d) for d in descs]
        if any(s is None for s in sides) or None in sides:
            skipped.append(("unparsed", joined[:90]))
            continue
        (ka, *a), (kb, *b) = sides
        net = sides[0][2] if ka == "track" else sides[0][4]
        if net is None or net_of(descs[1]) != net:
            skipped.append(("net-mismatch", joined[:90]))
            continue

        def endpoints(side):
            kind = side[0]
            if kind == "pad":
                _, ref, padname, layer, _n = side
                pad = pads.get((ref, padname))
                if pad is None:
                    return None
                p = pad.GetPosition()
                return [(p.x, p.y)], pad.GetLayer(), False
            # track: pick unused track matching net+length
            _, _net, length_mm, layer = side
            cands = [t for t in track_pool
                     if netname(t.GetNetCode()) == net
                     and abs(math.hypot(t.GetEnd().x - t.GetStart().x, t.GetEnd().y - t.GetStart().y) / MM - length_mm) < 0.02]
            if not cands:
                return None
            t = cands[0]
            return [(t.GetStart().x, t.GetStart().y), (t.GetEnd().x, t.GetEnd().y)], t.GetLayer(), t

        ea, eb = endpoints(sides[0]), endpoints(sides[1])
        if ea is None or eb is None:
            skipped.append(("endpoint-unresolved", joined[:90]))
            continue
        pa, layA, tA = ea
        pb, layB, tB = eb
        if layA != layB:
            skipped.append(("layer-change", joined[:90]))
            continue
        layer = layA
        # closest endpoint pair
        best = min(((ax, ay, bx, by) for ax, ay in pa for bx, by in pb),
                   key=lambda c: (c[0]-c[2])**2 + (c[1]-c[3])**2)
        x1, y1, x2, y2 = best
        width = int((0.4 if net in POWER_NETS else 0.2) * MM)
        hw = width / 2 + CLEARANCE * MM
        # edge guard
        if min(x1, x2) < OUT_MARGIN * MM or max(x1, x2) > (66.0 - OUT_MARGIN) * MM \
           or min(y1, y2) < OUT_MARGIN * MM or max(y1, y2) > (75.0 - OUT_MARGIN) * MM:
            skipped.append(("edge", joined[:90]))
            continue
        ps = capsule(x1, y1, x2, y2, hw)
        blocked = False
        for (pref, pname), pad in pads.items():
            padnet = netname(pad.GetNetCode())
            if padnet == net:
                continue
            if ps.Collide(pad.GetEffectiveShape(layer), 0):
                blocked = True
                break
        if not blocked:
            for t in tracks:
                tnet = netname(t.GetNetCode())
                if tnet == net or t is tA or t is tB:
                    continue
                if ps.Collide(t.GetEffectiveShape(layer), 0):
                    blocked = True
                    break
        if blocked:
            skipped.append(("blocked", joined[:90]))
            continue
        tr = pcbnew.PCB_TRACK(board)
        tr.SetStart(pcbnew.VECTOR2I(int(x1), int(y1)))
        tr.SetEnd(pcbnew.VECTOR2I(int(x2), int(y2)))
        tr.SetWidth(width)
        tr.SetLayer(layer)
        netinfo = board.FindNet(net)
        if netinfo is None:
            skipped.append(("net-missing", joined[:90]))
            continue
        tr.SetNet(netinfo)
        board.Add(tr)
        tracks.append(tr)
        for t in (tA, tB):
            if t in track_pool:
                track_pool.remove(t)
        accepted.append(f"{net} {joined[:80]}")

    print(f"accepted {len(accepted)} closures, skipped {len(skipped)}")
    for a in accepted:
        print("  +", a)
    for kind, s in skipped:
        print(f"  SKIP[{kind}]", s)
    if accepted:
        board.Save(board_path)
        print("saved")
    sys.exit(0 if not any(k == "unparsed" for k, _ in skipped) else 3)


if __name__ == "__main__":
    main()
