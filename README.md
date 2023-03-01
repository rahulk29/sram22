# SRAM 22

## A Configurable SRAM Generator

Sram22 parametrically generates SRAM blocks. At the moment, we only support the SKY130 process.
Sram22 is still a work in progress.

### Installation

#### BWRC

If you have BWRC access, you can install all features of Sram22. Make sure that you have SSH access to [bwrcrepo.eecs.berkeley.edu](https://bwrcrepo.eecs.berkeley.edu) from a BWRC machine by [adding your SSH key to your GitLab account](https://docs.gitlab.com/ee/user/ssh.html#add-an-ssh-key-to-your-gitlab-account). You will then need to add the following lines to your `~/.cargo/config.toml` file:

```
[net]
git-fetch-with-cli = true
```

You can then install Sram22 using the following commands:

```bash
git clone --recurse-submodules https://github.com/rahulk29/sram22.git
cd sram22/deps/Vlsir/bindings/python && pip install -e . && cd -
cd sram22/deps/Vlsir/VlsirTools && pip install -e . && cd -
cd sram22/sramgen && cargo install --all-features --path .
```

_Note: As Sram22 currently only supports the Sky130 process, you will need to have a signed Sky130 NDA on file to use certain features._

#### External

If you do not have BWRC access, you can still install Sram22, albeit without
the ability to invoke proprietary tools for DRC, LVS, PEX, and simulation.

Use the following commands:

```bash
git clone --recurse-submodules https://github.com/rahulk29/sram22.git
cd sram22/deps/Vlsir/bindings/python && pip install -e . && cd -
cd sram22/deps/Vlsir/VlsirTools && pip install -e . && cd -
cd sram22/sramgen && cargo install --path .
```

### Usage

```
sramgen 0.1.0
Rahul Kumar <rahulkumar@berkeley.edu>
A configurable SRAM generator

Usage: sramgen [OPTIONS]

Options:
  -c, --config <CONFIG>          Path to TOML configuration file [default: sramgen.toml]
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

Sram22 generates memory blocks based on a TOML configuration file. An example configuration, showing all the available options, is shown below:

```toml
num_words = 32
data_width = 32
mux_ratio = 2
write_size = 32
control = "ReplicaV1"
# The `pex_level` flag is only available with a full installation.
pex_level = "rcc"
```

To generate an SRAM using this configuration, put the above text into a file called
`sramgen_sram_32x32m2w8_replica_v1/sramgen.toml`, then run:

```
cd sramgen_sram_32x32m2w8_replica_v1
sramgen -o .
```

Add additional flags depending on what views you want to generate and what verification you want to run.
If you do not have access to BWRC servers, most flags will not be available.

If you have access to proprietary tools (eg. Calibre, Spectre, etc.) and would like access
to the Sram22 plugins for those tools, please contact us. Contact information is in `sramgen/Cargo.toml`.

The available configuration options are:
* `num_words`: Must be a power of 2, greater than or equal to 16.
* `data_width`: Must be a power of 2, greater than or equal to 16. Must be an integer multiple of `write_size`.
* `mux_ratio`: Must be 2, 4, or 8. A mux ratio of 2 is not recommended, as this option will be deprecated soon.
* `write_size`: Must be a power of 2, less than or equal to `data_width`.
* `control`: Must be `"ReplicaV1"`.
* `pex_level`: Must be `"r"`, `"c"`, `"rc"`, or `"rcc"`. If you do not have commercial plugins enabled, this option will be ignored.

### Technology Setup

See the `tech/sky130/` directory for an example of how to set up a new process to work with Sram22.


### Dependencies

In order to use Sram22, your system will need to have the following components:

- Rust (Sram22 is tested with version 1.65.0)
- Cmake
- Git v2.13+
- Python 3.8+

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be licensed under the BSD 3-Clause license,
without any additional terms or conditions.

