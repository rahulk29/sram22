// An interface for deterministic, open loop pattern generators.
interface det_patgen_if #(
    parameter MAX_ADDR,
    parameter ADDR_WIDTH = $clog2(MAX_ADDR),
    parameter DATA_WIDTH,
    parameter MASK_WIDTH
  ) (input clk);
  logic en;
  logic [ADDR_WIDTH-1:0] addr;
  logic [DATA_WIDTH-1:0] data;
  logic [DATA_WIDTH-1:0] check;
  logic [MASK_WIDTH-1:0] wmask;
  logic we, re, rst, done;
  modport slave (
    input clk, en, rst,
    output addr, data, check, wmask, we, re, done
  );

  assert property (@(posedge clk) disable iff (rst) (!(re && we)));
  assert property (@(posedge clk) disable iff (rst || en)
    (addr == $past(addr, 1)));
endinterface

typedef enum logic [2:0] {
  WRITE0, READ0, WRITE1, READ1, DONE
} zero_one_state_t;

module zero_one_patgen (
  det_patgen_if.slave intf
);
  logic [intf.ADDR_WIDTH-1:0] counter;
  zero_one_state_t state;

  always_ff @(posedge intf.clk) begin
    if (intf.rst) begin
      counter <= 0;
      state <= state.first();
    end
    else if (intf.en) begin
      if (counter == intf.ADDR_WIDTH'(intf.MAX_ADDR)) begin
        counter <= 0;
        state <= state.next();
      end else if (state != DONE) begin
        counter <= counter + 1;
      end
    end
  end

  always_comb begin
    intf.addr = counter;
    intf.data = state == WRITE1 ? {intf.DATA_WIDTH{1'b1}} : {intf.DATA_WIDTH{1'b0}};
    intf.check = state == READ1 ? {intf.DATA_WIDTH{1'b1}} : {intf.DATA_WIDTH{1'b0}};
    intf.wmask = {intf.MASK_WIDTH{1'b1}};
    intf.we = (state == WRITE0) || (state == WRITE1);
    intf.re = (state == READ0) || (state == READ1);
    intf.done = (state == DONE);
  end
endmodule

module counter #(parameter WIDTH = 8) (
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

