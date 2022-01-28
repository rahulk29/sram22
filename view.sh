#!/bin/bash
set -euf -o pipefail

cd _build && magic -T sky130A -d OGL &
