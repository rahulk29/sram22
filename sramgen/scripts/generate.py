import os
import vlsir
import vlsirtools.netlist as netlist
import sys

CKTS = [
    "and2",
    "bitcells_2x2",
    "bitcells_32x32",
    "decoder_16",
    "decoder_128",
    "precharge",
    "precharge_array",
    "sense_amp_array",
    "col_inv",
    "col_inv_array",
    "dout_buf",
    "dout_buf_array",
    "bitline_driver",
    "bitline_driver_array",
    "wordline_driver_array",
    "dff_array",
    "replica_bitcell_column",
    "replica_column",
    "sram_4x4m2",
    "sram_4x4m4",
    "sram_16x16m2",
    "sram_16x16m4",
    "sram_32x32m2",
    "sram_32x32m4",
    "sram_32x32m8",
    "sram_32x64",
    "sram_64x128",
    "sram_128x64",
]

PROPRIETARY_PRELUDE = """*SPICE NETLIST
* OPEN SOURCE CONVERSION PRELUDE

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
    os.makedirs("build/spice/", exist_ok=True)
    os.makedirs("build/ngspice/", exist_ok=True)

def generate(CKT):
    print(f"Generating {CKT}...")
    with open(f"build/pb/{CKT}.pb.bin", "rb") as f:
        tmp = f.read()
        with open(f"build/ngspice/{CKT}.spice", "w") as dest:
            print("\tngspice")
            inp = vlsir.spice_pb2.SimInput()
            inp.ParseFromString(tmp)
            netlist(pkg=inp.pkg, dest=dest, fmt="spice")
        with open(f"build/spice/{CKT}.spice", "w") as dest:
            print("\tspice")
            inp = vlsir.spice_pb2.SimInput()
            inp.ParseFromString(tmp)
            dest.write(PROPRIETARY_PRELUDE)
            netlist(pkg=inp.pkg, dest=dest, fmt="spice")
        print("\tDone!")

def netlist_all():
    for CKT in CKTS:
        generate(CKT)

if __name__ == "__main__":
    make_dirs()
    if len(sys.argv) < 2:
        netlist_all()
    else:
        generate(sys.argv[1])
