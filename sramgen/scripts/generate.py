import os
import vlsir
import vlsirtools.netlist as netlist

CKTS = [
    "and2",
    "bitcells",
    "bitcells_2x2",
    "decoder_16",
    "decoder_128",
    "precharge",
    "precharge_array",
    "sense_amp_array",
    "col_inv",
    "col_inv_array",
    "column_mux_4",
    "column_mux_4_array",
    "column_read_mux_2_array",
    "column_write_mux_2_array",
    "bitline_driver",
    "bitline_driver_array",
    "wordline_driver_array",
    "dff_array",
    "replica_bitcell_column",
    "replica_column",
    "sram_4x4",
    "sram_16x16",
    "sram_32x32",
    "sram_32x64",
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


def netlist_all():
    for CKT in CKTS:
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


if __name__ == "__main__":
    make_dirs()
    netlist_all()
