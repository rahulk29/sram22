import os
import vlsir
import vlsirtools.netlist as netlist
import sys

BUILD_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../build")

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


def make_dirs():
    os.makedirs(os.path.join(BUILD_DIR, "spice/"), exist_ok=True)
    os.makedirs(os.path.join(BUILD_DIR, "ngspice/"), exist_ok=True)
    os.makedirs(os.path.join(BUILD_DIR, "spectre/"), exist_ok=True)


def generate(CKT):
    print(f"Generating {CKT}...")
    with open(os.path.join(BUILD_DIR, f"pb/{CKT}.pb.bin"), "rb") as f:
        tmp = f.read()
        with open(os.path.join(BUILD_DIR, f"ngspice/{CKT}.spice"), "w") as dest:
            print("\tngspice")
            inp = vlsir.spice_pb2.SimInput()
            inp.ParseFromString(tmp)
            netlist(pkg=inp.pkg, dest=dest, fmt="spice")
        with open(os.path.join(BUILD_DIR, f"spice/{CKT}.spice"), "w") as dest:
            print("\tspice")
            inp = vlsir.spice_pb2.SimInput()
            inp.ParseFromString(tmp)
            dest.write(PROPRIETARY_PRELUDE_SPICE)
            netlist(pkg=inp.pkg, dest=dest, fmt="spice")
        with open(os.path.join(BUILD_DIR, f"spectre/{CKT}.spice"), "w") as dest:
            print("\tspectre-compatible spice")
            inp = vlsir.spice_pb2.SimInput()
            inp.ParseFromString(tmp)
            dest.write(PROPRIETARY_PRELUDE_SPECTRE)
            netlist(pkg=inp.pkg, dest=dest, fmt="spice")
        print("\tDone!")


if __name__ == "__main__":
    make_dirs()
    if len(sys.argv) < 2:
        print("Usage: python3 generate.py [CKT]")
    else:
        generate(sys.argv[1])
