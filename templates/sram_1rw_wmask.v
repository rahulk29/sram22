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
  clk,rstb,ce,we,wmask,addr,din,dout
);

  localparam DATA_WIDTH = {{data_width}};
  localparam ADDR_WIDTH = {{addr_width}};
  localparam WMASK_WIDTH = {{wmask_width}};
  localparam RAM_DEPTH = 1 << ADDR_WIDTH;

`ifdef USE_POWER_PINS
  inout vdd; // power
  inout vss; // ground
`endif
  input  clk; // clock
  input  rstb; // reset bar (active low reset)
  input  ce; // chip enable
  input  we; // write enable
  input [WMASK_WIDTH-1:0] wmask; // write mask
  input [ADDR_WIDTH-1:0]  addr; // address
  input [DATA_WIDTH-1:0]  din; // data in
  output reg [DATA_WIDTH-1:0] dout; // data out

  reg [DATA_WIDTH-1:0] mem [0:RAM_DEPTH-1];

  always @(posedge clk)
  begin
    if (ce && rstb) begin
      // Write
      if (we) begin
        {%- for i in range(end=wmask_width) -%}
          {% set lower = i * bits_per_mask %}
          {% set upper = (i + 1) * bits_per_mask - 1 -%}
          if (wmask[{{i}}]) begin
            mem[addr][{{upper}}:{{lower}}] <= din[{{upper}}:{{lower}}];
          end
        {%- endfor %}
      end

      // Read
      if (!we) begin
        dout <= mem[addr];
      end
    end
  end

endmodule

