import numpy as np
import matplotlib.pyplot as plt
from collections import defaultdict

interactive = False

saved = [
    "we",
    "wl",
    "bl",
    "br",
    "din",
    "din_b",
    "pc_b",
]


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
    with open("./bitcell_rw.dat") as f:
        return read_data(f)


def plot_data(data):
    plt.figure(dpi=300)
    plt.plot(data["time"], data["we"])
    plt.plot(data["time"], data["wl"])
    plt.plot(data["time"], data["bl"])
    plt.plot(data["time"], data["br"])
    plt.plot(data["time"], data["din"])
    plt.plot(data["time"], data["din_b"])
    plt.plot(data["time"], data["pc_b"])
    plt.legend(["we", "wl", "bl", "br", "din", "din_b", "pc_b"])
    plt.savefig("bitcell_rw.png")
    if interactive:
        plt.show()


if __name__ == "__main__":
    data = read_test_data()
    plot_data(data)
