# SRAM 22

## A Configurable SRAM Generator

SRAM22 parametrically generates SRAM blocks. At the moment, we only support the SKY130 process.
SRAM22 is still a work in progress.

### Installation

#### BWRC

If you have BWRC access, you can install all features of SRAM22. Make sure that you have SSH access to [bwrcrepo.eecs.berkeley.edu](https://bwrcrepo.eecs.berkeley.edu) from a BWRC machine by [adding your SSH key to your GitLab account](https://docs.gitlab.com/ee/user/ssh.html#add-an-ssh-key-to-your-gitlab-account). You will then need to add the following lines to your `~/.cargo/config.toml` file:

```
[net]
git-fetch-with-cli = true
```

You can then install SRAM22 using the following commands:

```bash
git clone https://github.com/rahulk29/sram22.git
cd sram22 && mv Cargo.bwrc.toml Cargo.toml && make install-all && cd -
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
      --pex                      Run PEX using Calibre
      --sim                      Run Spectre to verify SRAM functionality
  -a, --all                      Run all available steps
  -h, --help                     Print help information
  -V, --version                  Print version information
```

### Configuration

SRAM22 generates memory blocks based on a TOML configuration file. An example configuration, showing all the available options, is shown below:

```toml
num_words = 64
data_width = 32
mux_ratio = 4
write_size = 8
control = "ReplicaV2"
# The `pex_level` flag is only available with a full installation.
pex_level = "rcc"
```

To generate an SRAM using this configuration, put the above text into a file called
`sram22_64x32m4w8/sram22.toml`, then run:

```
cd sram22_64x32m4w8
sram22 -o .
```

Add additional flags depending on what views you want to generate and what verification you want to run.
If you do not have access to BWRC servers, most flags will not be available.

If you have access to proprietary tools (eg. Calibre, Spectre, etc.) and would like access
to the SRAM22 plugins for those tools, please contact us.

The number of rows in the SRAM bitcell array is `num_words / mux_ratio`.
The number of columns in the array is `data_width * mux_ratio`.

A valid configuration must have:
* A `mux_ratio` of 4 or 8
* A `data_width` that is an integer multiple of the `write_size`
* A power-of-two number of rows
* At least 16 rows
* At least 16 columns
* `control`: Must be `"ReplicaV2"`.
* `pex_level`: Must be `"r"`, `"c"`, `"rc"`, or `"rcc"`. If you do not have commercial plugins enabled, this option will be ignored.

### Dependencies

In order to use SRAM22, your system will need to have the following components:

- Rust (SRAM22 is tested with version 1.70.0)
- Make

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be licensed under the BSD 3-Clause license,
without any additional terms or conditions.

