"""Cross-check schematic netlist pin→net vs PCB pad→net (usage: crosscheck_pin_net.py <net.net> <board.kicad_pcb>).

Issue #3 acceptance: "Cross-check schematic pins against PCB pad nets".
Parses the KiCad exported netlist (.net) and the board's pad nets and prints
any mismatch. Test-point pads (TP*) are board-only by design (measurement
points added in layout) and excluded. The netlist prefixes hierarchical
local names with '/'; stripped before comparison. Exit 0 = clean.
"""
import re, sys


def find_blocks(text, token):
    """Yield the text of each balanced-paren token block (KiCad 9 = parens)."""
    for m in re.finditer(token, text):
        depth = 0
        i = m.start()
        while i < len(text):
            if text[i] == "(":
                depth += 1
            elif text[i] == ")":
                depth -= 1
                if depth == 0:
                    break
            i += 1
        yield text[m.start():i + 1]


net_txt = open(sys.argv[1]).read()
pcb_txt = open(sys.argv[2]).read()

# --- netlist side (block-split: node lines follow the net header) ---
mapping = {}
net_sections = re.split(r'(?=\(net \(code )', net_txt[net_txt.index("(nets"):])
for block in net_sections[1:]:
    nm = re.match(r'\(net \(code "\d+"\) \(name "([^"]*)"\)', block)
    if not nm:
        continue
    name = nm.group(1).lstrip("/")
    for ref, pin in re.findall(r'\(node \(ref "([^"]*)"\) \(pin "([^"]*)"\)', block):
        mapping[f"{ref}.{pin}"] = name

# --- board side ---
board_map = {}
for fpb in find_blocks(pcb_txt, r'\(footprint "'):
    rm = re.search(r'\(property "Reference" "([^"]*)"', fpb)
    if not rm:
        continue
    ref = rm.group(1)
    pads = [m.start() for m in re.finditer(r'\(pad "', fpb)]
    for idx, start in enumerate(pads):
        end = pads[idx + 1] if idx + 1 < len(pads) else len(fpb)
        seg = fpb[start:end]
        pm = re.match(r'\(pad "([^"]*)"', seg)
        nm = re.search(r'\(net \d+ "([^"]*)"\)', seg)
        if pm and nm:
            board_map[f"{ref}.{pm.group(1)}"] = nm.group(1)

mismatches = []
nc_checked = 0
net_freq = {}
for v in board_map.values():
    net_freq[v] = net_freq.get(v, 0) + 1
for k, v in sorted(mapping.items()):
    if k.startswith("TP"):
        continue
    b = board_map.get(k)
    if v.startswith("unconnected-("):
        # NC stub: schematic names it after the pin; board must show it as a
        # single-pin anonymous (or same-stub) net, i.e. no copper connectivity.
        nc_checked += 1
        if b is None or net_freq.get(b, 0) != 1:
            mismatches.append((k, f"NC-stub", b))
        continue
    if b != v:
        mismatches.append((k, v, b))

print(f"netlist pins checked: {len([k for k in mapping if not k.startswith('TP')])}")
print(f"NC stubs verified single-pin on board: {nc_checked}")
print(f"board pads mapped: {len(board_map)}")
print(f"mismatches: {len(mismatches)}")
for k, a, b in mismatches:
    print(f"  {k}: schematic={a!r} board={b!r}")
sys.exit(1 if mismatches else 0)
