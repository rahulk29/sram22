#!/usr/bin/bash

set -x

source /tools/B/rahulkumar/sky130/priv/drc/.bashrc

set -e

spectre -64 +spice + aps -format psfascii \
  {{ spice_path }} \
  -raw {{ raw_output_dir }} \
  =log {{ log_path }}
