Changes:
Separate wen/ren (should have defined behavior if both are high)
All SR latches, FFs, etc should have a reset signal
Write drivers should be inverters, not just single pull-down transistors
Write mux should be transmission gate rather than NMOS
Unify write/read mux
4-way banking, with control logic in the middle
Everything manually routed except non-scaling control logic
Divided WL + divided BL architecture
Top and bottom banks
During R/W, pick either top or bottom bank based on MSB of addr
Always read from both left+right halves of array
Col circuitry shared between top/bottom
latch input/output/inout pins
testbench should read/write every addr in pex sim and check internal state of sram bitcells
Write wordline pulse width controlled by inverter chain
Wordline gating happens after decoder

Open questions:
Why should there be a separate read/write mux
How to size predecoders
How to partition addr bits
How to layout predecoders
Where to place rbl
Do we need to support R90 variants?
Write assist circuitry?
Before turning on wl_en, must ensure predecoders/decoder have finished decoding. How should we figure out when to turn on wl_en? How should we minimize the time between when wl_en actually turns on and the theoretical earliest time at which wl_en could turn on? Do we assume that the delay through the driver chain for wl_en to all final stage gates is approximately equal to the decoder delay?

Bitcell organization:
4 banks: ul, ur, ll, lr
Words split across left and right banks
Only one of upper and lower banks activated at any time
Control logic inputs:
ce (chip enable)
we (write enable)
clk
reset
Control logic outputs:
sae
pc_b
colsel[i]/colsel_b[i]
wlen (rwl)
wrdrven

Read sequence of operations:
Latch addr/din
Disable precharge, sae
Decode address (turn on appropriate read mux config)
Turn on wlen pulse
Turn off wlen pulse once replica bitline reaches VDD/2
Send sae to trigger sense amp
Send sense amp outputs to DFFs/latches
Enable precharge

Write sequence of operations:
Latch addr/din
Disable precharge, sae
Decode address (turn on appropriate write mux config)
Turn on wrdrven/wlen pulse such that both worldline and bitline are driven at same time
Turn off wlen/wrdrven pulse after some inverter chain delay
Enable precharge

Control logic implementation:
SR latch for pc_b
Set at positive edge of clock
Reset inverter chain after sae goes high OR immediately after wlen goes low
SR latch for sae
Set when wlen goes low during read operation
Reset at positive edge of clock

```verilog
module controlv3 (
input ce, we, clk, reset,
output sae, pc_b, wlen, wrdrven,
inout rbl
)
  wire clkp;
  edge_detector clk_edge_detector(.clk(clk & ce), .clkp(clkp));

  sr_latch pc_b_latch(.s(clkp | reset), .r(sae #4), .q(pc_b));
  sr_latch sae_latch(.s(we_b & wlen_decoder), .r(clkp | reset), .q(sae));

  wire wlen_set, wlen_rst;
  assign wlen_set = clkp;
  assign wlen_rst = we ? clkp #4 : rbl_b;
  sr_latch wlen_latch(.s(wlen_set), .r(wlen_rst | reset), .q(wlen));

  wire wrdrven_rst;
  decoder_replica decoder_replica(.a(wlen_rst), .o(wlen_decoder));
  sr_latch wrdrven_latch(.s(clkp & we), .r(wlen_decoder | reset), .q(wrdrven));

endmodule
```
