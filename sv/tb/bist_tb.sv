`include "bist_if.sv"

module bist_tb;
  localparam int MaxAddr = 63;
  localparam int MuxRatio = 4;
  localparam int AddrWidth = $clog2(MaxAddr);
  localparam int DataWidth = 8;
  localparam int Rows = (MaxAddr + 1) / MuxRatio;
  localparam int RowWidth = $clog2(Rows);
  localparam int ColWidth = $clog2(MuxRatio);
  localparam int MaskWidth = 2;
  localparam int BlockWidth = DataWidth / MaskWidth;

  bit clk;
  always #5 clk = ~clk;
  initial begin
    clk = 0; end

  logic [DataWidth-1:0] sram_sim[MaxAddr:0]; // Simulated SRAM.
  logic success; // Denotes whether the data input to the BIST (what should be the output of the SRAM) should be correct.

  bist_if #(
      .MAX_ADDR  (MaxAddr),
      .DATA_WIDTH(DataWidth),
      .MASK_WIDTH(MaskWidth)
  ) if0 (
      .clk
  );
  bist #(.MUX_RATIO(MuxRatio)) dut (.intf(if0.bist));

  always_ff @(posedge if0.clk) begin
    if (if0.we) begin
      for (int i = 0; i < MaskWidth; i++) begin
        if (if0.wmask[i]) begin
          sram_sim[if0.addr][BlockWidth * i +: BlockWidth] <= if0.data[BlockWidth * i +: BlockWidth];
        end
      end
    end
    if (if0.re) begin
      if0.dout <= success ? sram_sim[if0.addr] : sram_sim[if0.addr] + 1;
    end
  end

  initial begin
    $dumpfile("bist_tb.vcd");
    $dumpvars;

    $display("Testing successful BIST run...");
    success = 1'b1;
    if0.rst = 1;
    if0.pattern_sel = bist_pattern::ZERO_ONE;
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
    if0.pattern_sel = bist_pattern::ZERO_ONE;
    repeat (16) @(posedge if0.clk);
    #1;
    if0.rst = 0;
    if0.en  = 1;

    @(posedge if0.done);

    @(negedge clk);
    assert (if0.done && if0.fail);

    $display("BIST correctly failed on invalid behavior. Testing reset to specific pattern...");
    success = 1'b0;
    if0.rst = 1;
    if0.pattern_sel = bist_pattern::MARCH_CM_ENHANCED;
    repeat (16) @(posedge if0.clk);
    #1;
    if0.rst = 0;
    if0.en  = 1;

    @(posedge if0.done);

    @(negedge clk);
    assert (if0.fail_pattern == bist_pattern::MARCH_CM_ENHANCED);
    assert (if0.done && if0.fail);

    $display("Test passed.");
    $finish;
  end
endmodule

