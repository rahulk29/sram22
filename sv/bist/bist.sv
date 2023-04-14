// An interface for deterministic pattern generators.
interface det_patgen_if #(
    parameter MAX_ADDR,
    parameter ADDR_WIDTH = $clog2(MAX_ADDR),
    parameter DATA_WIDTH,
    parameter MASK_WIDTH,
  ) (input clk);
  logic en;
  logic [ADDR_WIDTH-1:0] addr;
  logic [DATA_WIDTH-1:0] data;
  logic [DATA_WIDTH-1:0] check;
  logic [MASK_WIDTH-1:0] wmask;
  logic we, re;
  modport slave (
    input clk, en,
    output addr, data, check, wmask, we, re
  );
endinterface

module zero_one_patgen (
  det_patgen_if.slave intf
);
  assign intf.addr = 0;
  assign intf.data = 0;
  assign intf.check = 0;
  assign intf.we = 0;
  assign intf.re = 0;
endmodule

module counter #(
  input clk, en, rst,
  output logic [WIDTH-1:0] value
);
  always_ff @(posedge clk) begin
    if (rst) begin
      value <= 0;
    end else if (en) begin
      value <= value + 1;
    end
  end
endmodule

