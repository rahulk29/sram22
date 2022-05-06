************************************************************************
* auCdl Netlist:
* 
* Library Name:  AAA_Comp_SA
* Top Cell Name: AAA_Comp_SA_sense
* View Name:     schematic
* Netlisted on:  Mar 16 00:39:41 2022
************************************************************************

.INCLUDE  $BAG_TECH_CONFIG_DIR/calibre_setup/source.added
*.EQUATION
*.SCALE METER
.PARAM



************************************************************************
* Library Name: AAA_Comp_SA
* Cell Name:    AAA_Comp_SA_sense
* View Name:    schematic
************************************************************************

.SUBCKT AAA_Comp_SA_sense clk inn inp midn midp outn outp VDD VSS
*.PININFO clk:I inn:I inp:I midn:O midp:O outn:O outp:O VDD:B VSS:B
mXSWOP outp clk VDD VDD pshort m=1 w=1.00 l=0.15 mult=1 sa=0.0 sb=0.0 sd=0.0 
+ topography=normal area=0.063 perim=1.14
mXSWON outn clk VDD VDD pshort m=1 w=1.00 l=0.15 mult=1 sa=0.0 sb=0.0 sd=0.0 
+ topography=normal area=0.063 perim=1.14
mXSWMP midp clk VDD VDD pshort m=1 w=1.00 l=0.15 mult=1 sa=0.0 sb=0.0 sd=0.0 
+ topography=normal area=0.063 perim=1.14
mXSWMN midn clk VDD VDD pshort m=1 w=1.00 l=0.15 mult=1 sa=0.0 sb=0.0 sd=0.0 
+ topography=normal area=0.063 perim=1.14
mXPFBP outp outn VDD VDD pshort m=2 w=2.00 l=0.15 mult=1 sa=0.0 sb=0.0 sd=0.0 
+ topography=normal area=0.063 perim=1.14
mXPFBN outn outp VDD VDD pshort m=2 w=2.00 l=0.15 mult=1 sa=0.0 sb=0.0 sd=0.0 
+ topography=normal area=0.063 perim=1.14
mXTAIL tail clk VSS VSS nshort m=2 w=1.68 l=0.15 mult=1 sa=0.0 sb=0.0 sd=0.0 
+ topography=normal area=0.063 perim=1.14
mXNFBP outp outn midp VSS nshort m=2 w=1.68 l=0.15 mult=1 sa=0.0 sb=0.0 sd=0.0 
+ topography=normal area=0.063 perim=1.14
mXNFBN outn outp midn VSS nshort m=2 w=1.68 l=0.15 mult=1 sa=0.0 sb=0.0 sd=0.0 
+ topography=normal area=0.063 perim=1.14
mXINP midn inp tail VSS nshort m=2 w=1.68 l=0.15 mult=1 sa=0.0 sb=0.0 sd=0.0 
+ topography=normal area=0.063 perim=1.14
mXINN midp inn tail VSS nshort m=2 w=1.68 l=0.15 mult=1 sa=0.0 sb=0.0 sd=0.0 
+ topography=normal area=0.063 perim=1.14
.ENDS

