# Sram22 Column Circuitry

This is the order of components in an SRAM column, starting at the edge of the bitcell array and moving downwards:
* Precharge driver
* Read mux (PMOS)
* Write mux (NMOS)
* Sense amp
* Write driver
* Registers (not necessarily pitch matched)


# Read Mux

The read mux consists of 2 PMOS devices:

```
.SUBCKT column_read_mux_2 
+ vdd din_1 din_0 dout sel sel_b 

xMP0 
+ dout sel din_0 vdd 
+ sky130_fd_pr__pfet_01v8 
+ w='2.0' l='0.15' 

xMP1 
+ dout sel_b din_1 vdd 
+ sky130_fd_pr__pfet_01v8 
+ w='2.0' l='0.15' 

.ENDS
```

For each column, there are 2 muxes:
One to select between two adjacent BLs,
and the other to select between two adjacent BRs.

The inputs are the bitlines BL and BR.
The output `dout` is routed to the sense amps.

# Scratch

During read, BL at VDD, BR decreases from VDD.
Read mux sets bl-read to one of the two.


## Control Signals

All control signals go through a register first.
The description below refers to post-register signals.

Assume CS = 1.

```
WL_EN = !CLK
WRITE_EN = WE && !RBL_BL_DELAY && !CLK
S_EN = RBL_BL_DELAY && !CLK && !WE
PC = CLK && RBL_BL_DELAY
```

