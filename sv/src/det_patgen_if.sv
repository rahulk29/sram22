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
  modport slave(input clk, en, rst, output addr, data, check, wmask, we, re, done);

  assert property (@(posedge clk) disable iff (rst) (!(re && we)));
  assert property (@(posedge clk) disable iff (rst || en) (addr == $past(addr, 1)));
endinterface

