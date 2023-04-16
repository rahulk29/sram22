package march_cm_enhanced_state;
  typedef enum logic [3:0] {
    S1,
    S2,
    S3,
    S4,
    S5,
    S6,
    DONE
  } march_cm_enhanced_patgen_state_t;
endpackage

// Enhanced March C- pattern generator.
// Assumes columns form the LSB of the address.
module march_cm_enhanced_patgen #(
    parameter int MUX_RATIO = 4,
    parameter int MUX_BITS  = $clog2(MUX_RATIO)
) (
    det_patgen_if.slave intf
);
  import march_cm_enhanced_state::*;

  localparam int RowWidth = intf.ADDR_WIDTH - MUX_BITS;
  localparam int MaxRowAddr = (intf.MAX_ADDR + 1) / MUX_RATIO - 1;
  logic [RowWidth-1:0] row_addr;
  logic [MUX_BITS-1:0] col_addr;
  logic [1:0] op_ctr;
  logic row_min, col_max, col_min, d0, op_max, decrementing, row_done;
  march_cm_enhanced_patgen_state_t state;

  always_comb begin
    row_min = row_addr == 0;
    col_max = col_addr == MUX_BITS'(MUX_RATIO - 1);
    col_min = col_addr == 0;
    d0 = (state == S3) || (state == S5);
    op_max = op_ctr == 2'd3;
    decrementing = (state == S4 || state == S5);
    row_done = decrementing ? row_addr == 0 : row_addr == RowWidth'(MaxRowAddr);
  end

  always_ff @(posedge intf.clk) begin
    if (intf.rst) begin
      state <= state.first();
      row_addr <= 0;
      col_addr <= 0;
      op_ctr <= 0;
    end else if (intf.en && state != DONE) begin
      case (state)
        S1, S6: begin
          row_addr <= row_addr + 1;
          if (row_done) row_addr <= 0;
          if (row_done) col_addr <= col_addr + 1;
          if (row_done && col_max) begin
            col_addr <= 0;
            state <= state.next();
          end
        end
        S2, S3, S4, S5: begin
          op_ctr <= op_ctr + 1;

          if (op_max) begin
            if (decrementing) row_addr <= row_addr - 1;
            else row_addr <= row_addr + 1;
          end

          if (row_done && op_max) begin
            col_addr <= col_addr + 1;
            if (decrementing) row_addr <= RowWidth'(MaxRowAddr);
            else row_addr <= 0;
          end

          if (row_done && col_max && op_max) begin
            state <= state.next();
            col_addr <= 0;
            if (state == S3 || state == S4) row_addr <= RowWidth'(MaxRowAddr);
            else row_addr <= 0;
          end
        end
        default: begin
          $error("unreachable");
        end
      endcase
    end
  end

  always_comb begin
    intf.addr = {row_addr, col_addr};
    intf.we   = (state != S6) && ((state == S1) || op_ctr[0]);
    intf.re   = (state != S1) && ((state == S6) || !op_ctr[0]);

    intf.data = {intf.DATA_WIDTH{!d0}};
    if (state == S1) intf.data = {intf.DATA_WIDTH{1'b0}};

    intf.check = {intf.DATA_WIDTH{d0 ^ op_ctr[1]}};
    if (state == S6) intf.check = {intf.DATA_WIDTH{1'b0}};

    intf.done = (state == DONE);
  end

  always_comb begin
    // TODO
    intf.wmask = {intf.MASK_WIDTH{1'b1}};
  end
endmodule
