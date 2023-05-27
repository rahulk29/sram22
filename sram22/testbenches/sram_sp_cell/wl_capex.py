import numpy as np
import matplotlib.pyplot as plt
from collections import defaultdict

saved = ["ir", "ii"]


def read_data(f):
    data = defaultdict(lambda: [])
    for line in f.readlines():
        values = line.split()
        ctr = 0
        for key in saved:
            if ctr == 0:
                data["freq"].append(float(values[ctr]))
            ctr += 1
            data[key].append(float(values[ctr]))
            ctr += 1
    return {k: np.array(v) for k, v in data.items()}


def read_test_data():
    with open("./wl_capex.dat") as f:
        return read_data(f)


def analyze_data(data):
    f = data["freq"]
    imag = data["ii"]
    C = imag / (2 * np.pi * f)
    return np.average(C)


if __name__ == "__main__":
    data = read_test_data()
    C = analyze_data(data)
    print(f"Cell wordline capacitance: {C*1e15}fF")
