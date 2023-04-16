package zero_one_state;
  typedef enum logic [2:0] {
    WRITE0,
    READ0,
    WRITE1,
    READ1,
    DONE
  } zero_one_state_t;
endpackage

module zero_one_patgen (
    det_patgen_if.slave intf
);

  logic [intf.ADDR_WIDTH-1:0] counter;
  zero_one_state::zero_one_state_t state;

  always_ff @(posedge intf.clk) begin
    if (intf.rst) begin
      counter <= 0;
      state   <= state.first();
    end else if (intf.en) begin
      if (counter == intf.ADDR_WIDTH'(intf.MAX_ADDR)) begin
        counter <= 0;
        state   <= state.next();
      end else if (state != zero_one_state::DONE) begin
        counter <= counter + 1;
      end
    end
  end

  always_comb begin
    intf.addr = counter;
    intf.data = state == zero_one_state::WRITE1 ? {intf.DATA_WIDTH{1'b1}} : {intf.DATA_WIDTH{1'b0}};
    intf.check = state == zero_one_state::READ1 ? {intf.DATA_WIDTH{1'b1}} : {intf.DATA_WIDTH{1'b0}};
    intf.wmask = {intf.MASK_WIDTH{1'b1}};
    intf.we = (state == zero_one_state::WRITE0) || (state == zero_one_state::WRITE1);
    intf.re = (state == zero_one_state::READ0) || (state == zero_one_state::READ1);
    intf.done = (state == zero_one_state::DONE);
  end
endmodule

