// SRAM22 delay line model
// Output width: {{control_width}}

module {{module_name}}(
`ifdef USE_POWER_PINS
    vdd,
    vss,
`endif
    clk_in, clk_out, ctl, ctl_b
  );

  parameter CONTROL_WIDTH = {{control_width}} ;

`ifdef USE_POWER_PINS
    inout vdd; // power
    inout vss; // ground
`endif
  input  clk_in; // source clock
  output reg clk_out; // delayed clock
  input  [CONTROL_WIDTH-1:0] ctl; // control code (one-hot)
  input  [CONTROL_WIDTH-1:0] ctl_b; // complementary control code ("one-cold")


  always @(*) begin
    clk_out = #1 clk_in;
  end

endmodule

