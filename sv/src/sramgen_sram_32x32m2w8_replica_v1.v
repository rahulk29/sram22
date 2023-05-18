// SRAM22 SRAM model
// Words: 32
// Word size: 32
// Write size: 8

module sramgen_sram_32x32m2w8_replica_v1(
`ifdef USE_POWER_PINS
    vdd,
    vss,
`endif
    clk,we,wmask,addr,din,dout
  );

  // These parameters should NOT be set to
  // anything other than their defaults.
  parameter DATA_WIDTH = 32 ;
  parameter ADDR_WIDTH = 5 ;
  parameter WMASK_WIDTH = 4 ;
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
        if (wmask[0]) begin
          mem[addr][7:0] <= din[7:0];
        end
        if (wmask[1]) begin
          mem[addr][15:8] <= din[15:8];
        end
        if (wmask[2]) begin
          mem[addr][23:16] <= din[23:16];
        end
        if (wmask[3]) begin
          mem[addr][31:24] <= din[31:24];
        end

      // Output is arbitrary when writing to SRAM
      dout <= {DATA_WIDTH{1'bx}};
    end

    // Read
    if (!we) begin
      dout <= mem[addr];
    end
  end

endmodule

