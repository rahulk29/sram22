import numpy as np
import matplotlib.pyplot as plt
from collections import defaultdict

saved = [
    "din0",
    "din1",
    "din2",
    "din3",
    "sel0",
    "sel1",
    "sel_b0",
    "sel_b1",
    "dout",
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
    with open("./column_mux_4.dat") as f:
        return read_data(f)


def plot_data(data):
    plt.figure()
    plt.plot(data["time"], data["sel0"])
    plt.plot(data["time"], data["sel1"])
    plt.plot(data["time"], data["dout"])
    plt.legend(["sel0", "sel1", "dout"])
    plt.savefig("column_mux_4.png")


if __name__ == "__main__":
    data = read_test_data()
    plot_data(data)
