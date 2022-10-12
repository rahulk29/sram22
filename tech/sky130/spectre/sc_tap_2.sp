* SPICE NETLIST
***************************************
.SUBCKT Dpar d0 d1
.ENDS
***************************************
.SUBCKT sc_tap_2 VGND VPWR
** N=2 EP=2 IP=0 FDC=1
X0 VGND VPWR Dpar a=2.4991 p=6.41 m=1 $[nwdiode] $X=-190 $Y=1655 $D=185
.ENDS
***************************************
