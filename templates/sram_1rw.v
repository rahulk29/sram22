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

  localparam DATA_WIDTH = {{data_width}} ;
  localparam ADDR_WIDTH = {{addr_width}} ;
  localparam RAM_DEPTH = 1 << ADDR_WIDTH;

`ifdef USE_POWER_PINS
    inout vdd; // power
    inout vss; // ground
`endif
  input  clk; // clock
  input  rstb; // reset bar
  input  ce; // chip enable
  input  we; // write enable
  input [ADDR_WIDTH-1:0]  addr; // address
  input [DATA_WIDTH-1:0]  din; // data in
  output reg [DATA_WIDTH-1:0] dout; // data out

  reg [DATA_WIDTH-1:0] mem [0:RAM_DEPTH-1];

  // Fill memory with zeros.
  // For simulation only. The real SRAM
  // may not be initialized to all zeros.
  integer i;
  initial begin
    for (i = 0 ; i < RAM_DEPTH ; i = i + 1)
    begin
      mem[i] = {DATA_WIDTH{1'b0}};
    end
  end

  always @(posedge clk)
  begin
    if (!rstb) begin
        dout <= {DATAWIDTH{1'b1}};
    end else begin
      if (ce) begin 
        // Write
        if (we) begin
            mem[addr] <= din;
            // Output is all 1s when writing to SRAM due to precharge.
            dout <= {DATA_WIDTH{1'b1}};
        end

        // Read
        if (!we) begin
          dout <= mem[addr];
        end
      end
    end
  end

endmodule

