# Sram22 Verification

### Simulation

Sram22 is in need of additional functional verification.

### DRC

All components are DRC clean.
The entire SRAM without the power grid is DRC clean,
though the layout is not complete.

The power grid introduces additional DRC and LVS errors
that still need to be fixed.

Notable things that are missing from the layout include:
* Replica bitline
* Control logic
* Power strapping
* Top level routing

### LVS

The following are LVS clean:
* Precharge drivers
* Write muxes
* Column inverters
* Sense amplifiers

