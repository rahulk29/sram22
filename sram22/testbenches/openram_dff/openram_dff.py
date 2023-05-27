import numpy as np
import matplotlib.pyplot as plt
from collections import defaultdict

saved = [
    "clk",
    "d",
    "q",
    "qb",
]

plot = ["clk", "d", "q", "qb"]


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
    with open("./openram_dff.dat") as f:
        return read_data(f)


def plot_data(data):
    plt.figure()
    for key in plot:
        plt.plot(data["time"], data[key])
    plt.legend(plot)
    plt.savefig("openram_dff.png")


if __name__ == "__main__":
    data = read_test_data()
    plot_data(data)
