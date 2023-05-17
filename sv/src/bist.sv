`include "bist_if.sv"

package bist_state;
  typedef enum logic [3:0] {
    TEST,
    SUCCESS,
    FAILED
  } bist_state_t;
endpackage

module bist #(
    parameter int MUX_RATIO = 4,
    parameter int MUX_BITS  = $clog2(MUX_RATIO)
) (
    bist_if.bist intf
);
  import bist_pattern::*;

  bist_state::bist_state_t state;
  bist_pattern_t test_pattern;
  logic test_pattern_done;
  logic prev_re;
  logic [intf.DATA_WIDTH-1:0] prev_check;
  logic [intf.DATA_WIDTH-1:0] saved_dout;

  bist_if #(
      .MAX_ADDR  (intf.MAX_ADDR),
      .DATA_WIDTH(intf.DATA_WIDTH),
      .MASK_WIDTH(intf.MASK_WIDTH)
  ) zero_one_patgen_if (
      .clk(intf.clk)
  );

  bist_if #(
      .MAX_ADDR  (intf.MAX_ADDR),
      .DATA_WIDTH(intf.DATA_WIDTH),
      .MASK_WIDTH(intf.MASK_WIDTH)
  ) march_cm_enhanced_patgen_if (
      .clk(intf.clk)
  );

  zero_one_patgen zero_one_patgen0 (zero_one_patgen_if.patgen);

  march_cm_enhanced_patgen march_cm_enhanced_patgen0 (march_cm_enhanced_patgen_if.patgen);

  always_ff @(posedge intf.clk) begin
    prev_re <= intf.re;
    prev_check <= intf.check;
    if (intf.rst) begin
      state <= bist_state::TEST;
      test_pattern <= intf.pattern_sel;
    end else if (state == bist_state::TEST && intf.en) begin
      if (prev_re && intf.dout != prev_check) begin
        saved_dout <= intf.dout;
        state <= bist_state::FAILED;
      end
      if (test_pattern_done) begin
        if (test_pattern == test_pattern.last()) begin
          state <= bist_state::SUCCESS;
        end else begin
          test_pattern <= test_pattern.next();
        end
      end
    end
  end

  always_comb begin
    intf.actual = saved_dout;
    intf.done = state == bist_state::SUCCESS || state == bist_state::FAILED;
    intf.fail = state == bist_state::FAILED;
    intf.test_pattern = test_pattern;

    zero_one_patgen_if.en = test_pattern == ZERO_ONE && intf.en && state == bist_state::TEST;
    march_cm_enhanced_patgen_if.en = test_pattern == MARCH_CM_ENHANCED && intf.en && state == bist_state::TEST;
    zero_one_patgen_if.rst = intf.rst;
    march_cm_enhanced_patgen_if.rst = intf.rst;

    case (test_pattern)
      ZERO_ONE: begin
        test_pattern_done = zero_one_patgen_if.done;
        intf.addr = zero_one_patgen_if.addr;
        intf.data = zero_one_patgen_if.data;
        intf.wmask = zero_one_patgen_if.wmask;
        intf.we = zero_one_patgen_if.we;
        intf.re = zero_one_patgen_if.re;
        intf.check = zero_one_patgen_if.check;
      end
      MARCH_CM_ENHANCED: begin
        test_pattern_done = march_cm_enhanced_patgen_if.done;
        intf.addr = march_cm_enhanced_patgen_if.addr;
        intf.data = march_cm_enhanced_patgen_if.data;
        intf.wmask = march_cm_enhanced_patgen_if.wmask;
        intf.we = march_cm_enhanced_patgen_if.we;
        intf.re = march_cm_enhanced_patgen_if.re;
        intf.check = march_cm_enhanced_patgen_if.check;
      end
      default: begin
        test_pattern_done = 1'b0;
        intf.addr = intf.ADDR_WIDTH'(1'b0);
        intf.data = intf.DATA_WIDTH'(1'b0);
        intf.wmask = intf.MASK_WIDTH'(1'b0);
        intf.we = 1'b0;
        intf.re = 1'b0;
        intf.check = intf.DATA_WIDTH'(1'b0);
      end
    endcase
  end
endmodule

