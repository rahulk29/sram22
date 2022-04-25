import numpy as np
import matplotlib.pyplot as plt
from collections import defaultdict

saved = [
    "clk",
    "sae_in",
    "sae_out",
    "f0",
    "f1",
    "f2",
    "f3",
    "f4",
    "f5",
    "f6",
]

plot = ["clk", "sae_in", "sae_out", "f1", "f2", "f3", "f4", "f5"]


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
    with open("./tmc.dat") as f:
        return read_data(f)


def plot_data(data):
    plt.figure()
    for key in plot:
        plt.plot(data["time"], data[key])
    plt.legend(plot)
    plt.xlabel("time")
    plt.savefig("tmc.png")


if __name__ == "__main__":
    data = read_test_data()
    plot_data(data)
