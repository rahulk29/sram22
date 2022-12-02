# SRAM 22

## A Configurable SRAM Generator

Sram22 parametrically generates SRAM blocks. At the moment, we only support the SKY130 process.
Sram22 is still a work in progress.

### Installation

To set up the CLI, run the following commands:

```bash
git clone https://github.com/rahulk29/sram22.git
cd sram22/sramgen
cargo install --all-features --path .
```

### Usage

```
sramgen 0.1.0
Rahul Kumar <rahulkumar@berkeley.edu>
A configurable SRAM generator

Usage: sramgen [OPTIONS]

Options:
  -c, --config <CONFIG>          Path to TOML configuration file [default: sramgen.toml]
  -o, --output-dir <OUTPUT_DIR>  Directory in which to write output files
  -q, --quick                    Skip long running steps (DRC, LVS, LEF generation, etc.)
  -h, --help                     Print help information
  -V, --version                  Print version information
```

### Configuration

Sram22 generates memory blocks based on a TOML configuration file. An example configuration, showing all the available options, is shown below:

```toml
num_words = 32
data_width = 32
mux_ratio = 2
write_size = 32
control= "ReplicaV1"
```

### Technology Setup

See the `tech/sky130/` directory for an example of how to set up a new process to work with Sram22.


### Dependencies

In order to use Sram22, your system will need to have the following components:

- Rust (Sram22 is tested with version 1.65.0)
- Cmake

