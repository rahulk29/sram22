// SRAM22 SRAM model
// Words: {{num_words}}
// Word size: {{data_width}}
// Write size: {{data_width}}

module {{module_name}}(
`ifdef USE_POWER_PINS
    vdd,
    vss,
`endif
    clk,rstb,ce,we,addr,din,dout
  );

  localparam DATA_WIDTH = {{data_width}};
  localparam ADDR_WIDTH = {{addr_width}};
  localparam RAM_DEPTH = 1 << ADDR_WIDTH;

`ifdef USE_POWER_PINS
    inout vdd; // power
    inout vss; // ground
`endif
  input  clk; // clock
  input  rstb; // reset bar (active low reset)
  input  ce; // chip enable
  input  we; // write enable
  input [ADDR_WIDTH-1:0]  addr; // address
  input [DATA_WIDTH-1:0]  din; // data in
  output reg [DATA_WIDTH-1:0] dout; // data out

  reg [DATA_WIDTH-1:0] mem [0:RAM_DEPTH-1];

  always @(posedge clk)
  begin
    if (ce && rstb) begin
      // Write
      if (we) begin
          mem[addr] <= din;
      end

      // Read
      if (!we) begin
        dout <= mem[addr];
      end
    end
  end

endmodule

