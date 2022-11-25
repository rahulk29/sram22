import os
import vlsir
import vlsirtools.netlist as netlist
import sys
import argparse

parser = argparse.ArgumentParser(
    prog="generate", description="generate netlists from VLSIR binary files"
)

parser.add_argument("binary_path", help="Path to VLSIR binary file")
parser.add_argument(
    "-o", "--output_dir", help="directory where output files should be written"
)

PROPRIETARY_PRELUDE_SPECTRE = """*SPICE NETLIST
* OPEN SOURCE CONVERSION PRELUDE (SPECTRE)

.SUBCKT sky130_fd_pr__special_nfet_pass d g s b
.PARAM w=1.0 l=1.0 mult=1
M0 d g s b npass l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__special_nfet_latch d g s b
.PARAM w=1.0 l=1.0 mult=1
M0 d g s b npd l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__nfet_01v8 d g s b
.PARAM w=1.0 l=1.0 mult=1
M0 d g s b nshort l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__pfet_01v8 d g s b
.PARAM w=1.0 l=1.0 mult=1
M0 d g s b pshort l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__special_pfet_pass d g s b
.PARAM w=1.0 l=1.0 mult=1
M0 d g s b ppu l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__pfet_01v8_hvt d g s b
.PARAM w=1.0 l=1.0 mult=1
M0 d g s b phighvt l='l' w='w' mult='mult'
.ENDS
"""

PROPRIETARY_PRELUDE_SPICE = """*SPICE NETLIST
* OPEN SOURCE CONVERSION PRELUDE (SPICE)

.SUBCKT sky130_fd_pr__special_nfet_pass d g s b PARAMS: w=1.0 l=1.0 mult=1
M0 d g s b npass l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__special_nfet_latch d g s b PARAMS: w=1.0 l=1.0 mult=1
M0 d g s b npd l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__nfet_01v8 d g s b PARAMS: w=1.0 l=1.0 mult=1
M0 d g s b nshort l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__pfet_01v8 d g s b PARAMS: w=1.0 l=1.0 mult=1
M0 d g s b pshort l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__special_pfet_pass d g s b PARAMS: w=1.0 l=1.0 mult=1
M0 d g s b ppu l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__pfet_01v8_hvt d g s b PARAMS: w=1.0 l=1.0 mult=1
M0 d g s b phighvt l='l' w='w' mult='mult'
.ENDS
"""


def generate(bin_path, output_dir):
    if output_dir is not None:
        os.makedirs(output_dir, exist_ok=True)
    else:
        output_dir = os.path.dirname(os.path.abspath(bin_path))
    print(f"Generating netlist for binary at {bin_path}...")
    with open(bin_path, "rb") as f:
        tmp = f.read()
        CKT = os.path.basename(bin_path).split(".")[0]
        with open(os.path.join(output_dir, f"{CKT}.ngspice.spice"), "w") as dest:
            print("\tngspice")
            inp = vlsir.spice_pb2.SimInput()
            inp.ParseFromString(tmp)
            netlist(pkg=inp.pkg, dest=dest, fmt="spice")
        with open(os.path.join(output_dir, f"{CKT}.spice"), "w") as dest:
            print("\tspice")
            inp = vlsir.spice_pb2.SimInput()
            inp.ParseFromString(tmp)
            dest.write(PROPRIETARY_PRELUDE_SPICE)
            netlist(pkg=inp.pkg, dest=dest, fmt="spice")
        with open(os.path.join(output_dir, f"{CKT}.spectre.spice"), "w") as dest:
            print("\tspectre-compatible spice")
            inp = vlsir.spice_pb2.SimInput()
            inp.ParseFromString(tmp)
            dest.write(PROPRIETARY_PRELUDE_SPECTRE)
            netlist(pkg=inp.pkg, dest=dest, fmt="spice")
        print("\tDone!")


if __name__ == "__main__":
    args = parser.parse_args()
    generate(args.binary_path, args.output_dir)
