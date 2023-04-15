
module counter_tb;
  localparam WIDTH = 12;

  bit clk;
  reg rst = 1;
  reg en = 0;
  wire [WIDTH-1:0] value;

  always #10 clk = ~clk;
  initial begin
    clk = 0;
  end

  counter #(.WIDTH(WIDTH)) dut (.clk, .en, .rst, .value);

  initial begin
    $dumpfile("counter_tb.vcd");
    $dumpvars;
    repeat (16) @(posedge clk);
    rst = 0;
    repeat (16) @(posedge clk);
    assert (value == 0);
    en = 1;
    repeat (16) @(posedge clk);
    assert (value == 16);
    repeat (16) @(posedge clk);
    assert (value == 32);
    $finish;
  end
endmodule

