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
cd sram22 && make install-all && cd -
```

_Note: As SRAM22 currently only supports the Sky130 process, you will need to have a signed Sky130 NDA on file to use certain features._

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
sram22 0.1.0
Rahul Kumar <rahulkumar@berkeley.edu>
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
num_words = 32
data_width = 32
mux_ratio = 2
write_size = 32
control = "ReplicaV2"
# The `pex_level` flag is only available with a full installation.
pex_level = "rcc"
```

To generate an SRAM using this configuration, put the above text into a file called
`sram22_32x32m2w8/sram22.toml`, then run:

```
cd sram22_32x32m2w8
sram22 -o .
```

Add additional flags depending on what views you want to generate and what verification you want to run.
If you do not have access to BWRC servers, most flags will not be available.

If you have access to proprietary tools (eg. Calibre, Spectre, etc.) and would like access
to the SRAM22 plugins for those tools, please contact us. Contact information is in `sram22/Cargo.toml`.

The available configuration options are:
* `num_words`: Must be a power of 2, greater than or equal to 16.
* `data_width`: Must be a power of 2, greater than or equal to 16. Must be an integer multiple of `write_size`.
* `mux_ratio`: Must be 4, or 8.
* `write_size`: Must be a power of 2, less than or equal to `data_width`.
* `control`: Must be `"ReplicaV2"`.
* `pex_level`: Must be `"r"`, `"c"`, `"rc"`, or `"rcc"`. If you do not have commercial plugins enabled, this option will be ignored.

### Technology Setup

See the `tech/sky130/` directory for an example of how to set up a new process to work with SRAM22.

### Dependencies

In order to use SRAM22, your system will need to have the following components:

- Rust (SRAM22 is tested with version 1.69.0)
- Make

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be licensed under the BSD 3-Clause license,
without any additional terms or conditions.

