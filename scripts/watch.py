#!/usr/bin/python3

import os
import webbrowser

command = "cargo r --release -- configs/sram_16x16.toml"
out_dir = "_build/sram_16x16/"
cell = "sram_16x16.mag"
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
