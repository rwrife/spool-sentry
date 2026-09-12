"""Import a Specctra SES session and SAVE IMMEDIATELY (usage: import_ses.py <board> <ses>).

Pitfall #17: ImportSpecctraSES mutates only in memory — a probe raising before
Save() loses the whole route. Save first, then report metrics on a reload.
"""
import sys
import pcbnew

board = pcbnew.LoadBoard(sys.argv[1])
ok = pcbnew.ImportSpecctraSES(board, sys.argv[2])
print("import returned:", ok)
board.Save(sys.argv[1])
b2 = pcbnew.LoadBoard(sys.argv[1])
tracks = list(b2.GetTracks())
segs = [t for t in tracks if type(t).__name__ == "PCB_TRACK"]
vias = [t for t in tracks if type(t).__name__ == "PCB_VIA"]
print(f"imported+saved: {len(segs)} tracks, {len(vias)} vias")
