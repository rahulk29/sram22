# SRAM 22

## A Configurable SRAM Generator

Sram22 parametrically generates SRAM blocks. At the moment, we only support the SKY130 process.

### Usage

```
sram22 0.1.0
Rahul Kumar <rahulkumar@berkeley.edu>
A configurable SRAM generator

USAGE:
    sram22 <CONFIG>

ARGS:
    <CONFIG>    Path to a TOML configuration file specifying memory options

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information
```

### Configuration

Sram22 generates memory blocks based on a TOML configuration file. An example configuration, showing all the available options, is shown below:

```toml
rows = 16
cols = 16
output_dir = "../_build/sram_16x16/"
tech_dir = "../tech/sky130/magic"
```

### Technology Setup

See the `tech/sky130/` directory for an example of how to set up a new process to work with Sram22.


### Dependencies

In order to use Sram22, your system will need to have the following components:

- Rust (Sram22 is tested with version 1.58.1)
- Magic (version 8.3.X)

