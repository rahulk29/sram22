#!/usr/bin/python3

import argparse
import gdstk


def getcell(cells, name):
    for cell in cells:
        print(cell.name)
        if cell.name == name:
            return cell
    return None


def gds2svg(input, cell, output):
    print(f"gds2svg {input} > {output}")
    lib = gdstk.read_gds(input)
    cell = getcell(lib.cells, cell)
    cell.write_svg(output, scaling=50000)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Convert a GDSII file to an SVG")
    parser.add_argument("-i", "--input", required=True, help="the input file")
    parser.add_argument(
        "-c", "--cell", required=True, help="the cell to convert to SVG"
    )
    parser.add_argument("-o", "--output", required=False, help="The output file")
    args = parser.parse_args()
    gds2svg(args.input, args.cell, args.output)
