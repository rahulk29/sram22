package bist_state;
  typedef enum logic [3:0] {
    TEST,
    DONE,
    FAILED,
  } bist_state_t;
endpackage

module bist #(
  parameter int MUX_RATIO = 4,
  parameter int MUX_BITS = $clog2(MUX_RATIO)
) (
  det_patgen_if.bist intf
);
  import bist_pattern_sel::*;

  bist_state::bist_state_t state;
  bist_pattern_sel_t test_pattern;

  always_ff @(posedge intf.clk) begin
    if (intf.rst) begin
    end
  end
endmodule

