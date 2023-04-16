module counter #(
    parameter int WIDTH = 8
) (
    input clk,
    en,
    rst,
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

