module march_cm_enhanced_tb;
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

  det_patgen_if #(
      .MAX_ADDR  (MaxAddr),
      .DATA_WIDTH(DataWidth),
      .MASK_WIDTH(2)
  ) if0 (
      .clk
  );
  march_cm_enhanced_patgen #(.MUX_RATIO(MuxRatio)) dut (.intf(if0.slave));

  initial begin
    $dumpfile("march_cm_enhanced_tb.vcd");
    $dumpvars;

    if0.rst = 1;
    repeat (16) @(posedge if0.clk);
    #1;
    if0.rst = 0;
    if0.en  = 1;

    // State 1
    for (int c = 0; c < MuxRatio; c++) begin
      for (int r = 0; r < Rows; r++) begin
        @(negedge clk);
        assert (if0.addr == {RowWidth'(r), ColWidth'(c)})
        else $error("Got incorrect address: %d", if0.addr);
        assert (if0.we)
        else $error("expected write enable");
        assert (if0.data == {DataWidth{1'b0}});
      end
    end
    $display("Stage 1 OK.");

    // State 2
    for (int c = 0; c < MuxRatio; c++) begin
      for (int r = 0; r < Rows; r++) begin
        for (int i = 0; i < 4; i++) begin
          @(negedge clk);
          assert (if0.addr == {RowWidth'(r), ColWidth'(c)})
          else
            $error(
                "Got incorrect address: %d, expected %d (r=%d, c=%d, i=%d)",
                if0.addr,
                {
                  RowWidth'(r), ColWidth'(c)
                },
                r,
                c,
                i
            );
          if (i == 0 || i == 2) begin
            assert (if0.re);

            if (i == 0) begin
              assert (if0.check == {DataWidth{1'b0}});
            end else begin
              assert (if0.check == {DataWidth{1'b1}});
            end
          end else begin
            assert (if0.we);
            assert (if0.data == {DataWidth{1'b1}});
          end
        end
      end
    end
    $display("Stage 2 OK.");

    // State 3
    for (int c = 0; c < MuxRatio; c++) begin
      for (int r = 0; r < Rows; r++) begin
        for (int i = 0; i < 4; i++) begin
          @(negedge clk);
          assert (if0.addr == {RowWidth'(r), ColWidth'(c)})
          else $error("Got incorrect address: %d", if0.addr);
          if (i == 0 || i == 2) begin
            assert (!if0.we && if0.re);

            if (i == 0) begin
              assert (if0.check == {DataWidth{1'b1}});
            end else begin
              assert (if0.check == {DataWidth{1'b0}});
            end
          end else begin
            assert (if0.we && !if0.re);
            assert (if0.data == {DataWidth{1'b0}});
          end
        end
      end
    end
    $display("Stage 3 OK.");

    // State 4
    for (int c = 0; c < MuxRatio; c++) begin
      for (int r = Rows - 1; r >= 0; r--) begin
        for (int i = 0; i < 4; i++) begin
          @(negedge clk);
          assert (if0.addr == {RowWidth'(r), ColWidth'(c)})
          else
            $display(
                "Got incorrect address: got %d, expected %d at time %0t",
                if0.addr,
                {
                  RowWidth'(r), ColWidth'(c)
                },
                $time
            );
          if (i == 0 || i == 2) begin
            assert (!if0.we && if0.re);

            if (i == 0) begin
              assert (if0.check == {DataWidth{1'b0}});
            end else begin
              assert (if0.check == {DataWidth{1'b1}});
            end
          end else begin
            assert (if0.we && !if0.re);
            assert (if0.data == {DataWidth{1'b1}});
          end
        end
      end
    end
    $display("Stage 4 OK.");

    // State 5
    for (int c = 0; c < MuxRatio; c++) begin
      for (int r = Rows - 1; r >= 0; r--) begin
        for (int i = 0; i < 4; i++) begin
          @(negedge clk);
          assert (if0.addr == {RowWidth'(r), ColWidth'(c)})
          else
            $display(
                "Got incorrect address: got %d, expected %d at time %0t",
                if0.addr,
                {
                  RowWidth'(r), ColWidth'(c)
                },
                $time
            );
          if (i == 0 || i == 2) begin
            assert (!if0.we && if0.re);

            if (i == 0) begin
              assert (if0.check == {DataWidth{1'b1}});
            end else begin
              assert (if0.check == {DataWidth{1'b0}});
            end
          end else begin
            assert (if0.we && !if0.re);
            assert (if0.data == {DataWidth{1'b0}});
          end
        end
      end
    end
    $display("Stage 5 OK.");

    // State 6
    for (int c = 0; c < MuxRatio; c++) begin
      for (int r = 0; r < Rows; r++) begin
        @(negedge clk);
        assert (if0.addr == {RowWidth'(r), ColWidth'(c)})
        else $error("Got incorrect address: %d", if0.addr);
        assert (if0.re)
        else $error("expected read enable");
        assert (if0.check == {DataWidth{1'b0}});
      end
    end
    $display("Stage 6 OK.");

    @(negedge clk);
    assert (if0.done);
    $display("Test passed.");
    $finish;
  end
endmodule

