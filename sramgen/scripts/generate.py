import os
import vlsir
import vlsirtools.netlist as netlist

inp = vlsir.spice_pb2.SimInput()

CKTS = [
    "and2",
    "bitcells",
    "decoder_16",
    "precharge",
    "precharge_array",
    "sense_amp_array",
    "column_mux_4",
    "column_mux_4_array",
    "bitline_driver",
    "bitline_driver_array",
    "wordline_driver_array",
    "dff_array",
    "sram_4x4",
    "sram_16x16",
]


def make_dirs():
    os.makedirs("build/spice/", exist_ok=True)


def netlist_all():
    for CKT in CKTS:
        with open(f"build/pb/{CKT}.pb.bin", "rb") as f:
            dest = open(f"build/spice/{CKT}.spice", "w")
            tmp = f.read()
            inp.ParseFromString(tmp)
            netlist(pkg=inp.pkg, dest=dest, fmt="spice")
            dest.close()
        print(f"generated {CKT}")


if __name__ == "__main__":
    make_dirs()
    netlist_all()
