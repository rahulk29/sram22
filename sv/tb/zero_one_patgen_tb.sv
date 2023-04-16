
module zero_one_patgen_tb;
  localparam int MaxAddr = 31;
  localparam int AddrWidth = $clog2(MaxAddr);
  localparam int DataWidth = 8;
  bit clk;
  always #10 clk = ~clk;
  initial begin
    clk = 0;
  end

  det_patgen_if #(
      .MAX_ADDR  (MaxAddr),
      .DATA_WIDTH(DataWidth),
      .MASK_WIDTH(2)
  ) if0 (
      .clk
  );
  zero_one_patgen dut (.intf(if0.slave));

  initial begin
    $dumpfile("zero_one_patgen_tb.vcd");
    $dumpvars;
    if0.rst = 1;
    repeat (16) @(posedge if0.clk);
    #1;
    if0.rst = 0;
    if0.en  = 1;

    // Write 0
    for (int i = 0; i <= MaxAddr; i++) begin
      @(posedge if0.clk);
      assert (if0.addr == AddrWidth'(i))
      else $error("Wrong address: expected %d, got %d at time %0t", i, if0.addr, $time);
      assert (if0.data == {DataWidth{1'b0}});
      assert (if0.we);
      assert (!if0.re);
    end

    // Read 0
    for (int i = 0; i <= MaxAddr; i++) begin
      @(posedge if0.clk);
      assert (if0.addr == AddrWidth'(i));
      assert (if0.check == {DataWidth{1'b0}});
      assert (!if0.we);
      assert (if0.re);
    end

    // Write 1
    for (int i = 0; i <= MaxAddr; i++) begin
      @(posedge if0.clk);
      assert (if0.addr == AddrWidth'(i));
      assert (if0.data == {DataWidth{1'b1}});
      assert (if0.we);
      assert (!if0.re);
    end

    // Read 1
    for (int i = 0; i <= MaxAddr; i++) begin
      @(posedge if0.clk);
      assert (if0.addr == AddrWidth'(i));
      assert (if0.check == {DataWidth{1'b1}});
      assert (!if0.we);
      assert (if0.re);
    end


    repeat (532) @(posedge if0.clk);
    assert (if0.done);
    $display("Test passed.");
    $finish;
  end
endmodule

