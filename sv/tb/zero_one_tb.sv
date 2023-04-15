
module zero_one_tb;
  bit clk;
  always #10 clk = ~clk;
  initial begin
    clk = 0;
  end

  det_patgen_if #(
      .MAX_ADDR  (127),
      .DATA_WIDTH(8),
      .MASK_WIDTH(2)
  ) if0 (
      .clk
  );
  zero_one_patgen dut (.intf(if0.slave));

  initial begin
    $dumpfile("zero_one_tb.vcd");
    $dumpvars;
    if0.rst = 1;
    repeat (16) @(posedge if0.clk);
    if0.rst = 0;
    if0.en  = 1;
    repeat (532) @(posedge if0.clk);
    assert (if0.done);
    $finish;
  end
endmodule

