* SPICE NETLIST
***************************************
.SUBCKT Dpar d0 d1
.ENDS
***************************************
.SUBCKT sc_inv_2 VNB VPB A VPWR Y VGND
** N=6 EP=6 IP=0 FDC=5
M0 Y A VGND VNB nshort L=0.15 W=0.84 m=1 r=5.6 a=0.126 p=1.98 mult=1 $X=410 $Y=345 $D=9
M1 VGND A Y VNB nshort L=0.15 W=0.84 m=1 r=5.6 a=0.126 p=1.98 mult=1 $X=840 $Y=345 $D=9
M2 Y A VPWR VPB phighvt L=0.15 W=1.26 m=1 r=8.4 a=0.189 p=2.82 mult=1 $X=410 $Y=1835 $D=89
M3 VPWR A Y VPB phighvt L=0.15 W=1.26 m=1 r=8.4 a=0.189 p=2.82 mult=1 $X=840 $Y=1835 $D=89
X4 VNB VPB Dpar a=3.3943 p=7.37 m=1 $[nwdiode] $X=-190 $Y=1655 $D=185
.ENDS
***************************************
