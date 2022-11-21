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

  // Update registers
  always @(posedge clk)
  begin
    we_reg <= we;
    addr_reg <= addr;
    din_reg <= din;

    // Output is precharged to VDD for first half clock cycle
    dout <= {DATA_WIDTH{1'b1}};
  end

  // Write
  always @ (negedge clk)
  begin : MEM_WRITE
    if (we_reg) begin
        mem[addr_reg] <= din_reg;

        // Output is arbitrary when writing to SRAM
        dout <= {DATA_WIDTH{1'bx}};
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

