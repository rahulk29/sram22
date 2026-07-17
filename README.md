# SRAM 22

## A Configurable SRAM Generator

SRAM22 parametrically generates SRAM blocks. At the moment, we only support the SKY130 process.
SRAM22 is still a work in progress.

### Dependencies

In order to use SRAM22, your system will need to have the following components:

- Rust (SRAM22 is tested with version 1.81.0)
- Make
- A local clone of our [slightly modified version of the SKY 130 PDK](https://github.com/ucb-substrate/skywater-pdk). 
You will also need to set the environment variable `SKY130_OPEN_PDK_ROOT` to the absolute path of the local PDK's root directory.
Substrate uses standard cells from the `sky130_fd_sc_hs` library, so you will also need to run the following from the PDK root directory:
    ```
    git submodule update --init libraries/sky130_fd_sc_hs/latest
    ```

### Installation

#### BWRC

If you have BWRC access, you can install all features of SRAM22. Make sure that you have SSH access to [bwrcrepo.eecs.berkeley.edu](https://bwrcrepo.eecs.berkeley.edu) from a BWRC machine by [adding your SSH key to your GitLab account](https://docs.gitlab.com/ee/user/ssh.html#add-an-ssh-key-to-your-gitlab-account). You will then need to add the following lines to your `~/.cargo/config.toml` file:

```
[net]
git-fetch-with-cli = true
[registries]
substrate = { index = "https://github.com/substrate-labs/crates-index" }
```

You can then install SRAM22 using the following commands:

```bash
git clone https://github.com/rahulk29/sram22.git
cd sram22 && mv Cargo.bwrc.toml Cargo.toml && make install && cd -
```

#### External

If you do not have BWRC access, you can still install SRAM22, albeit without
the ability to invoke proprietary tools for DRC, LVS, PEX, and simulation.

Use the following commands:

```bash
git clone https://github.com/rahulk29/sram22.git
cd sram22 && make install && cd -
```

### Usage

```
sram22 0.2.0
Rahul Kumar <rahulkumar@berkeley.edu>, Rohan Kumar <rohankumar@berkeley.edu>
A configurable SRAM generator

Usage: sram22 [OPTIONS]

Options:
  -c, --config <CONFIG>          Path to TOML configuration file [default: sram22.toml]
  -o, --output-dir <OUTPUT_DIR>  Directory to which output files should be saved
      --lef                      Generate LEF (used in place and route)
      --lib                      Generate LIB (setup, hold, and delay timing information)
      --drc                      Run DRC using Calibre
      --lvs                      Run LVS using Calibre
  -a, --all                      Run all available steps
  -h, --help                     Print help information
  -V, --version                  Print version information
```

### Configuration

SRAM22 generates memory blocks based on a TOML configuration file. Configurations are specified
as an array of `[[sram]]` configurations, allowing up to multiple SRAMs to be generated.

```toml
[[sram]]
num_words = 64
data_width = 32
mux_ratio = 4
write_size = 8

[[sram]]
num_words = 256
data_width = 64
mux_ratio = 4
write_size = 8
# The `pex_level` flag is only available with a full installation.
pex_level = "rcc"
```

Save this as `sram22.toml` and run:

```
sram22
```

Each `[[sram]]` block is generated independently. Output files are placed in subdirectories
named after the SRAM (e.g. `build/sram22_64x32m4w8/`)

The number of rows in the SRAM bitcell array is `num_words / mux_ratio`.
The number of columns in the array is `data_width * mux_ratio`.

A valid configuration must have:
* A `mux_ratio` of 4 or 8
* A `data_width` that is an integer multiple of the `write_size`
* A power-of-two number of rows
* At least 16 rows
* At least 16 columns
* `pex_level` (optional): Must be `"r"`, `"c"`, `"rc"`, or `"rcc"`. Only available with a full installation. When set, PEX runs automatically — no additional flag required. Each `[[sram]]` block sets its own `pex_level` independently; SRAMs without it skip PEX, and `--lib` falls back to the plain SPICE netlist for those entries.

### LIB generation

The `--lib` flag generates Liberty (.lib) timing files for the tt/ss/ff PVT corners.
In a full BWRC installation the LIB is produced by Liberate MX running SPICE simulation.
If a config has `pex_level` set, PEX runs first and Liberate MX uses the extracted netlist;
otherwise Liberate MX falls back to the plain SPICE netlist.
Without a full installation the LIB is produced by an open-source interpolation model;
interpolated LIBs carry a conservative overestimate of up to 2% for SRAM configurations
with `data_width` between 8 and 128.  `data_width` outside that range is not supported by
the open-source model and will produce an error.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be licensed under the BSD 3-Clause license,
without any additional terms or conditions.

