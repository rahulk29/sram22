#!/usr/bin/python3

import os
import webbrowser

command = "cargo r"
out_dir = "_build/"
cell = "nand2_pm_sh.mag"
outsvg = "out.svg"

script = f'''
magic -T sky130A -d XR -noconsole <<EOF
load {cell}
select top cell
expand
findbox zoom
select clear
plot svg {outsvg}
quit
EOF
'''

os.system(command)
os.chdir(out_dir)
cwd = os.getcwd()
os.system(script)
webbrowser.open(f'file://{cwd}/{outsvg}')
