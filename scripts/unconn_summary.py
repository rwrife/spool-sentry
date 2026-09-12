"""Summarize DRC unconnected_items by net and endpoint kind (usage: unconn_summary.py <drc.json>)."""
import json, sys, re
from collections import Counter

d = json.load(open(sys.argv[1]))
un = d.get("unconnected_items", [])
nets = Counter()
kinds = Counter()
pairs = []
for u in un:
    descs = [it.get("description", "") for it in u.get("items", [])]
    netnames = []
    for s in descs:
        m = re.search(r"\[/?([^\]]+)\]", s)
        if m:
            netnames.append(m.group(1))
        kinds[re.split(r"[\[/]", s.strip())[0]] += 1
    nets[tuple(sorted(set(netnames)))] += 1
    pairs.append(descs)
print("total unconnected pairs:", len(un))
print("endpoint kinds:", dict(kinds))
print("by net:")
for k, v in nets.most_common():
    print(f"  {k}: {v}")
print("\nsample endpoints:")
for p in pairs[:12]:
    print("  ", " | ".join(p))
