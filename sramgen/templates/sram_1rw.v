// SRAM22 SRAM model
// Words: {{num_words}}
// Word size: {{data_width}}
// Write size: {{data_width}}

module {{module_name}}(
`ifdef USE_POWER_PINS
    vdd,
    vss,
`endif
    clk,we,addr,din,dout
  );

  parameter DATA_WIDTH = {{data_width}} ;
  parameter ADDR_WIDTH = {{addr_width}} ;
  parameter RAM_DEPTH = 1 << ADDR_WIDTH;

`ifdef USE_POWER_PINS
    inout vdd;
    inout vss;
`endif
  input  clk; // clock
  input  we; // write enable
  input [ADDR_WIDTH-1:0]  addr;
  input [DATA_WIDTH-1:0]  din;
  output reg [DATA_WIDTH-1:0] dout;

  reg  we_reg;
  reg [ADDR_WIDTH-1:0]  addr_reg;
  reg [DATA_WIDTH-1:0]  din_reg;
  reg [DATA_WIDTH-1:0]  dout;

  reg [DATA_WIDTH-1:0] mem [0:RAM_DEPTH-1];

  // Update registers
  always @(posedge clk)
  begin
    we_reg <= we;
    addr_reg <= addr;
    din_reg <= din;
    dout <= { {{data_width}} {1'b1}};
  end

  // Write
  always @ (negedge clk)
  begin : MEM_WRITE
    if (we_reg) begin
        mem[addr_reg] <= din_reg;
    end
  end

  // Read
  always @ (negedge clk)
  begin : MEM_READ
    if (!we_reg) begin
       dout <= mem[addr_reg];
     end
  end

endmodule

