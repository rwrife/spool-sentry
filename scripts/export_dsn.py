"""Export the board to Specctra DSN for the autorouter (usage: export_dsn.py <board> <dsn>).

ExportSpecctraDSN is module-level in KiCad 9 pcbnew (not a BOARD method).
"""
import sys
import pcbnew

board = pcbnew.LoadBoard(sys.argv[1])
pcbnew.ExportSpecctraDSN(board, sys.argv[2])
print("exported", sys.argv[2])
