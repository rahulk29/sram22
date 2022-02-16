#!/usr/bin/python3

import os
import webbrowser

command = "cargo r"
out_dir = "_build/"
cell = "sram_8x8.mag"
outsvg = "out.svg"

script = f"""
magic -T sky130A -d XR -noconsole <<EOF
load {cell}
select top cell
expand
findbox zoom
select clear
plot svg {outsvg}
quit -noprompt
EOF
"""

os.system(command)
os.chdir(out_dir)
cwd = os.getcwd()
os.system(script)
webbrowser.open(f"file://{cwd}/{outsvg}")
