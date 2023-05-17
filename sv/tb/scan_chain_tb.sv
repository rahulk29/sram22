`include "scan_chain.sv"

module scan_chain_tb;
  localparam int N = 64;

  bit clk;
  always #5 clk = ~clk;
  initial begin
    clk = 0;
  end

  logic [N-1:0] expected_value;

  scan_chain_if #(.N) if0 (.scan_clk(clk));
  scan_chain dut (.intf(if0.scan_chain));

  always_comb begin
    if0.rst_din = N'(1'b0);
  end

  initial begin
    $dumpfile("scan_chain_tb.vcd");
    $dumpvars;

    if0.scan_rstb = 0;
    repeat (16) @(posedge if0.scan_clk);
    #1;
    if0.scan_rstb = 1;
    if0.scan_en = 1;
    if0.scan_in = 1;
    expected_value = 0;

    repeat (N + 1) begin
      @(negedge if0.scan_clk);
      assert (if0.scan_out == 1'b0);
      assert (if0.dout == expected_value);
      expected_value = (expected_value << 1) + 1;
    end

    if0.scan_in = 0;

    repeat (N) begin
      @(negedge if0.scan_clk);
      expected_value = (expected_value << 1);
      assert (if0.scan_out == 1'b1);
      assert (if0.dout == expected_value);
    end

    if0.scan_in = 1;

    repeat (5) begin
      @(negedge if0.scan_clk);
      expected_value = (expected_value << 1) + 1;
      assert (if0.scan_out == 1'b0);
      assert (if0.dout == expected_value);
    end

    if0.scan_en = 0;

    repeat (5) begin
      @(negedge if0.scan_clk);
      assert (if0.dout == expected_value);
    end

    if0.scan_rstb = 0;
    repeat (16) @(negedge if0.scan_clk);
    assert (if0.dout == N'(1'b0));

    @(negedge clk);

    $display("Test passed.");
    $finish;
  end
endmodule

