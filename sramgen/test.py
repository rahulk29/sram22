import vlsir
import vlsirtools.netlist as netlist

inp = vlsir.spice_pb2.SimInput()

with open("hi.bin", "rb") as f:
    dest = open("netlist.scs", "w")
    tmp = f.read()
    inp.ParseFromString(tmp)
    netlist(pkg=inp.pkg, dest=dest, fmt="spectre")
    dest.close()


