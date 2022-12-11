{% set bits_per_mask = data_width / wmask_width -%}
// SRAM22 SRAM model
// Words: {{num_words}}
// Word size: {{data_width}}
// Write size: {{ bits_per_mask }}

module {{module_name}}(
`ifdef USE_POWER_PINS
    vdd,
    vss,
`endif
    clk,we,wmask,addr,din,dout
  );

  // These parameters should NOT be set to
  // anything other than their defaults.
  parameter DATA_WIDTH = {{data_width}} ;
  parameter ADDR_WIDTH = {{addr_width}} ;
  parameter WMASK_WIDTH = {{wmask_width}} ;
  parameter RAM_DEPTH = 1 << ADDR_WIDTH;

`ifdef USE_POWER_PINS
    inout vdd; // power
    inout vss; // ground
`endif
  input  clk; // clock
  input  we; // write enable
  input [WMASK_WIDTH-1:0] wmask; // write mask
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
    // Write
    if (we) begin
      {%- for i in range(end=wmask_width) -%}
        {% set lower = i * bits_per_mask %}
        {% set upper = (i + 1) * bits_per_mask - 1 -%}
        if (wmask[{{i}}]) begin
          mem[addr][{{upper}}:{{lower}}] <= din[{{upper}}:{{lower}}];
        end
      {%- endfor %}

      // Output is arbitrary when writing to SRAM
      dout <= {DATA_WIDTH{1'bx}};
    end

    // Read
    if (!we) begin
      dout <= mem[addr];
    end
  end

endmodule

