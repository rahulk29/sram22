
module tb;
  bit clk;
  always #10 clk = ~clk;
  initial begin
    clk <= 0;
  end

  det_patgen_if #(.MAX_ADDR(128), .DATA_WIDTH(8)) if0 (.clk);
  zero_one_patgen dut (.intf(if0.slave));

  initial begin
    repeat (5) @(posedge if0.clk);
    $finish;
  end
endmodule

