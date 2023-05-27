import numpy as np
import matplotlib.pyplot as plt
from collections import defaultdict

saved = [
    "clk",
    "dout_3",
    "dout_2",
    "dout_1",
    "dout_0",
    "pc_b",
    "wl_en",
    "wr_drv_en",
]

saved.extend([f"q_{i}" for i in range(8)])

plot = ["clk", "dout_0", "dout_1"]


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
