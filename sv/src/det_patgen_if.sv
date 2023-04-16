// An interface for deterministic, open loop pattern generators.
interface det_patgen_if #(
    parameter int MAX_ADDR,
    parameter int ADDR_WIDTH = $clog2(MAX_ADDR),
    parameter int DATA_WIDTH,
    parameter int MASK_WIDTH
) (
    input clk
);
  logic en;
  logic [ADDR_WIDTH-1:0] addr;
  logic [DATA_WIDTH-1:0] data;
  logic [DATA_WIDTH-1:0] check;
  logic [MASK_WIDTH-1:0] wmask;
  logic we, re, rst, done;

  // Pattern generator modport.
  modport slave(input clk, en, rst, output addr, data, check, wmask, we, re, done);

  // Single port memories cannot read and write simultaneously.
  assert property (@(posedge clk) disable iff (rst) (!(re && we)));

  // Address and data should be held constant when enable is low.
  assert property (@(posedge clk) disable iff (rst || en) (addr == $past(addr, 1)));
  assert property (@(posedge clk) disable iff (rst || en) (data == $past(data, 1)));
endinterface

