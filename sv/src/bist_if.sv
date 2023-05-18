`ifndef BIST_IF_DONE
`define BIST_IF_DONE
package bist_pattern;
  typedef enum logic [3:0] {
    ZERO_ONE,
    MARCH_CM_ENHANCED
  } bist_pattern_t;
endpackage

// An interface for BIST components.
interface bist_if #(
    parameter int MAX_ADDR,
    parameter int ADDR_WIDTH  = $clog2(MAX_ADDR),
    parameter int DATA_WIDTH,
    parameter int MASK_WIDTH,
    parameter int CYCLE_WIDTH = 64
) (
    input clk
);
  import bist_pattern::*;

  logic en;
  logic [ADDR_WIDTH-1:0] addr, fail_addr;
  logic [DATA_WIDTH-1:0] data, expected, dout, actual, fail_expected, fail_actual;
  logic [ MASK_WIDTH-1:0] wmask;
  logic [CYCLE_WIDTH-1:0] fail_cycle;
  bist_pattern_t pattern_sel, fail_pattern;

  logic we, re, rst, done, fail;

  // Pattern generator modport.
  modport patgen(input clk, en, rst, output addr, data, wmask, we, re, expected, done);

  // BIST modport.
  modport bist(
      input clk, en, rst, dout, pattern_sel,
      output addr, data, wmask, we, re, fail_addr, fail_expected, fail_actual, fail_pattern, fail_cycle, done, fail
  );

  // Single port memories cannot read and write simultaneously.
  assert property (@(posedge clk) disable iff (rst) (!(re && we)));

  // Address and data should be held constant when enable is low.
  assert property (@(posedge clk) disable iff (rst || en) (addr == $past(addr, 1)));
  assert property (@(posedge clk) disable iff (rst || en) (data == $past(data, 1)));
endinterface
`endif
