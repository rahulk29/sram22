{% set bits_per_mask = data_width / wmask_width %}
// SRAM22 SRAM model
// Words: {{num_words}}
// Word size: {{data_width}}
// Write size: {{wmask_width}}

module {{module_name}}(
`ifdef USE_POWER_PINS
    vdd,
    vss,
`endif
    clk,we,wmask,addr,din,dout
  );

  parameter DATA_WIDTH = {{data_width}} ;
  parameter ADDR_WIDTH = {{addr_width}} ;
  parameter WMASK_WIDTH = {{wmask_width}} ;
  parameter RAM_DEPTH = 1 << ADDR_WIDTH;

`ifdef USE_POWER_PINS
    inout vdd;
    inout vss;
`endif
  input  clk; // clock
  input  we; // write enable
  input [WMASK_WIDTH-1:0] wmask;
  input [ADDR_WIDTH-1:0]  addr;
  input [DATA_WIDTH-1:0]  din;
  output reg [DATA_WIDTH-1:0] dout;

  reg  we_reg;
  reg [WMASK_WIDTH-1:0] wmask_reg;
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

  always @(posedge clk)
  begin
    // Write
    if (we_reg) begin
      {% for i in range(end=wmask_width) %}
        {% set lower = i * bits_per_mask %}
        {% set upper = (i + 1) * bits_per_mask - 1 %}
        if (wmask_reg[{{i}}]) begin
          mem[addr_reg][{{upper}}:{{lower}}] <= din_reg[{{upper}}:{{lower}}];
        end
      {% endfor %}

      // Output is arbitrary when writing to SRAM
      dout <= {DATA_WIDTH{1'bx}};
    end

    // Read
    if (!we_reg) begin
      dout <= mem[addr_reg];
    end

    // Update registers
    we_reg <= we;
    wmask_reg <= wmask;
    addr_reg <= addr;
    din_reg <= din;
  end

endmodule

