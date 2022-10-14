* SPICE NETLIST
* * NMOS
.SUBCKT sky130_fd_pr__nfet_01v8 d g s b PARAMS: w=1.0 l=1.0
M0 d g s b nshort l={l} w={w}
.ENDS
* * PMOS
.SUBCKT sky130_fd_pr__pfet_01v8 d g s b PARAMS: w=1.0 l=1.0
M0 d g s b pshort l={l} w={w}
.ENDS
