* SPICE NETLIST
.SUBCKT Dpar d0 d1
.ENDS
.SUBCKT sc_and2_2 VNB VPB A B VPWR X VGND
** N=9 EP=7 IP=0 FDC=9
M0 8 A 5 VNB nshort L=0.15 W=0.42 m=1 r=2.8 a=0.063 p=1.14 mult=1 $X=495 $Y=235 $D=9
M1 VGND B 8 VNB nshort L=0.15 W=0.42 m=1 r=2.8 a=0.063 p=1.14 mult=1 $X=855 $Y=235 $D=9
M2 X 5 VGND VNB nshort L=0.15 W=0.84 m=1 r=5.6 a=0.126 p=1.98 mult=1 $X=1380 $Y=235 $D=9
M3 VGND 5 X VNB nshort L=0.15 W=0.84 m=1 r=5.6 a=0.126 p=1.98 mult=1 $X=1810 $Y=235 $D=9
M4 5 A VPWR VPB phighvt L=0.15 W=0.42 m=1 r=2.8 a=0.063 p=1.14 mult=1 $X=425 $Y=1985 $D=89
M5 VPWR B 5 VPB phighvt L=0.15 W=0.42 m=1 r=2.8 a=0.063 p=1.14 mult=1 $X=855 $Y=1985 $D=89
M6 X 5 VPWR VPB phighvt L=0.15 W=1.26 m=1 r=8.4 a=0.189 p=2.82 mult=1 $X=1380 $Y=1835 $D=89
M7 VPWR 5 X VPB phighvt L=0.15 W=1.26 m=1 r=8.4 a=0.189 p=2.82 mult=1 $X=1810 $Y=1835 $D=89
X8 VNB VPB Dpar a=5.1847 p=9.29 m=1 $[nwdiode] $X=-190 $Y=1655 $D=185
.ENDS
***************************************
