from protos.edatool.sim.v1.sim_pb2 import SimulationData
import matplotlib.pyplot as plt

with open("./scripts/simdata.bin", "rb") as f:
    d = SimulationData()
    d.ParseFromString(f.read())

tran = d.analyses[0]
t = tran.values["sweep_var"].real.v
y = tran.values["y"].real.v
a = tran.values["a"].real.v
b = tran.values["b"].real.v

plt.plot(t, a)
plt.plot(t, b)
plt.plot(t, y)
plt.show()
