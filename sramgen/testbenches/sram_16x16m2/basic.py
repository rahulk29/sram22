import numpy as np
import matplotlib.pyplot as plt
from collections import defaultdict

saved = [
    "clk",
    "we",
    "dout_3",
    "dout_2",
    "dout_1",
    "dout_0",
    "pc_b",
    "wl_en",
    "wr_drv_en",
    "sense_amp_en",
    "bl_4",
    "bl_5",
    "bl_6",
    "bl_7",
    "br_4",
    "br_5",
    "br_6",
    "br_7",
    "bl_out_0",
    "br_out_0",
    "bl_out_1",
    "br_out_1",
    "bl_out_2",
    "br_out_2",
    "bl_out_3",
    "br_out_3",
]

plot = ["dout_0", "dout_1", "dout_2", "clk"]


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
    with open("./basic.dat") as f:
        return read_data(f)


def plot_data(data):
    plt.figure(dpi=150)
    for key in plot:
        plt.plot(data["time"], data[key])
    plt.legend(plot)
    plt.savefig("basic.png")
    plt.show()


if __name__ == "__main__":
    data = read_test_data()
    plot_data(data)
