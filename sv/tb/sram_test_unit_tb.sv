`include "sram_test_unit.sv"

module sram_test_unit_tb;
  localparam int MaxAddr = 31;
  localparam int MuxRatio = 2;
  localparam int AddrWidth = $clog2(MaxAddr);
  localparam int DataWidth = 32;
  localparam int MaskWidth = 4;

  bit clk, scan_clk, clk_gate;
  always #5 clk = ~clk && clk_gate;
  always #5 scan_clk = ~scan_clk;
  initial begin
    clk = 0;
    clk_gate = 1;
  end

  sram_test_unit_if if0 (
      .clk,
      .scan_clk
  );
  sram_test_unit #(
    .MAX_ADDR(MaxAddr),
    .DATA_WIDTH(DataWidth),
    .MASK_WIDTH(MaskWidth),
    .MUX_RATIO(MuxRatio)
  ) dut (.intf(if0.sram_test_unit));

  initial begin
    $dumpfile("bist_tb.vcd");
    $dumpvars;

    $display("Testing successful BIST run...");
    if0.bist_rst = 1;
    if0.bist_pattern_sel = bist_pattern::ZERO_ONE;
    if0.sram_ctl_sel = sram_ctl_src::BIST;
    if0.sram_ctl_scan_en = 0;
    if0.sram_ctl_scan_rstb = 0;
    if0.dout_scan_en = 0;
    repeat (16) @(posedge if0.clk);
    #1;
    if0.bist_rst = 0;
    if0.bist_en  = 1;
    if0.sram_ctl_scan_rstb = 1;
    assert (~if0.bist_done);

    @(posedge if0.bist_done);
    assert (if0.bist_done && ~if0.bist_fail);

    clk_gate = 0;

    $display("Test passed.");
    $finish;
  end
endmodule

