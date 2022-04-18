import vlsir
import vlsirtools.netlist as netlist

inp = vlsir.spice_pb2.SimInput()

CKT = "decoder"

print("hi")

with open(f"build/{CKT}.pb.bin", "rb") as f:
    dest = open(f"build/{CKT}.spice", "w")
    tmp = f.read()
    inp.ParseFromString(tmp)
    netlist(pkg=inp.pkg, dest=dest, fmt="spice")
    dest.close()


