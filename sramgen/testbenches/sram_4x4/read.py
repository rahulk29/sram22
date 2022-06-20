import numpy as np
import matplotlib.pyplot as plt
from collections import defaultdict

saved = [
    "clk",
    "clk_b",
    "dout_out_1",
    "dout_out_0",
    "wl_3",
    "wl_2",
    "wl_1",
    "wl_0",
    "bl_2",
    "br_2",
    "bl_0",
    "br_0",
    "sae",
    "blr_1",
    "brr_1",
    "blr_0",
    "brr_0",
    "addr_0",
    "dout_1",
    "dout_0",
    "dout_negedge_1",
    "dout_negedge_0",
]

plot = ["dout_out_0", "dout_out_1"]


def read_data(f):
    data = defaultdict(lambda: [])
    for line in f.readlines():
        values = line.split()
        ctr = 0
        for key in saved:
            if ctr == 0:
                data["time"].append(float(values[ctr]))
            ctr += 1
            data[key].append(float(values[ctr]))
            ctr += 1
    return {k: np.array(v) for k, v in data.items()}


def read_test_data():
    with open("./read.dat") as f:
        return read_data(f)


def plot_data(data):
    plt.figure(dpi=150)
    for key in plot:
        plt.plot(data["time"], data[key])
    plt.legend(plot)
    plt.savefig("read.png")
    plt.show()


if __name__ == "__main__":
    data = read_test_data()
    plot_data(data)
