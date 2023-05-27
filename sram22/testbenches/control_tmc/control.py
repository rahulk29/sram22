import numpy as np
import matplotlib.pyplot as plt
from collections import defaultdict

saved = [
    "clk",
    "cs",
    "we",
    "sae_i",
    "pc",
    "pc_b",
    "wl_en",
    "write_driver_en",
    "sense_en",
    "rbl",
    "rbr",
]

plot = ["wl_en", "sae_i", "sense_en"]


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
    with open("./control.dat") as f:
        return read_data(f)


def plot_data(data):
    plt.figure(dpi=150)
    for sig in plot:
        plt.plot(data["time"], data[sig])
    plt.legend(plot)
    plt.savefig("control.png")
    plt.show()


if __name__ == "__main__":
    data = read_test_data()
    time = data["time"]
    plot_data(data)
