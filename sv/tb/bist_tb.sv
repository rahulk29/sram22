`include "bist_if.sv"

module bist_tb;
  localparam int MaxAddr = 63;
  localparam int MuxRatio = 4;
  localparam int AddrWidth = $clog2(MaxAddr);
  localparam int DataWidth = 8;
  localparam int Rows = (MaxAddr + 1) / MuxRatio;
  localparam int RowWidth = $clog2(Rows);
  localparam int ColWidth = $clog2(MuxRatio);

  bit clk;
  always #5 clk = ~clk;
  initial begin
    clk = 0;
  end

  logic [DataWidth-1:0] next_dout; // Stores the previous check value in order to successfully simulate desired SRAM behavior, or stores an invalid value to simulate a failure mode.
  logic success; // Denotes whether the data input to the BIST (what should be the output of the SRAM) should be correct.

  bist_if #(
      .MAX_ADDR  (MaxAddr),
      .DATA_WIDTH(DataWidth),
      .MASK_WIDTH(2)
  ) if0 (
      .clk
  );
  bist #(.MUX_RATIO(MuxRatio)) dut (.intf(if0.bist));

  always_ff @(posedge if0.clk) begin
    prev_check <= success ? if0.check : if0.check + 1'b1;
  end

  always_comb begin
    if0.dout = prev_check;
  end

  initial begin
    $dumpfile("bist_tb.vcd");
    $dumpvars;

    $display("Testing successful BIST run...");
    success = 1'b1;
    if0.rst = 1;
    if0.pattern_sel = bist_pattern_sel::ZERO_ONE;
    repeat (16) @(posedge if0.clk);
    #1;
    if0.rst = 0;
    if0.en  = 1;

    @(posedge if0.done);

    @(negedge clk);
    assert (if0.done && ~if0.fail);

    $display("Sucessful BIST run complete. Testing failure mode...");
    success = 1'b0;
    if0.rst = 1;
    if0.pattern_sel = bist_pattern_sel::ZERO_ONE;
    repeat (16) @(posedge if0.clk);
    #1;
    if0.rst = 0;
    if0.en  = 1;

    @(posedge if0.fail);

    @(negedge clk);
    assert (~if0.done && if0.fail);

    $display("BIST correctly failed on invalid behavior. Testing reset to specific pattern...");
    success = 1'b0;
    if0.rst = 1;
    if0.pattern_sel = bist_pattern_sel::MARCH_CM_ENHANCED;
    repeat (16) @(posedge if0.clk);
    #1;
    if0.rst = 0;
    if0.en  = 1;

    @(posedge if0.fail);

    @(negedge clk);
    assert(if0.test_pattern == bist_pattern_sel::MARCH_CM_ENHANCED);
    assert (~if0.done && if0.fail);

    $display("Test passed.");
    $finish;
  end
endmodule

