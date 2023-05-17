`ifndef SCAN_CHAIN_DONE
`define SCAN_CHAIN_DONE
interface scan_chain_if #(
    parameter int N
) (
    input scan_clk
);
  logic scan_in;
  logic scan_out;
  logic scan_en;
  logic scan_rstb;
  logic [N-1:0] dout;
  logic [N-1:0] rst_din;

  modport scan_chain(input scan_clk, scan_in, scan_en, scan_rstb, rst_din, output scan_out, dout);
endinterface

module scan_chain (
    scan_chain_if.scan_chain intf
);

  logic [intf.N-1:0] shift_reg;
  always_ff @(posedge intf.scan_clk or negedge intf.scan_rstb) begin
    if (~intf.scan_rstb) begin
      shift_reg <= intf.rst_din;
    end else if (intf.scan_en) begin
      intf.scan_out <= shift_reg[intf.N-1];
      shift_reg <= {{shift_reg[intf.N-2:0]}, {intf.scan_in}};
    end else begin
      shift_reg <= shift_reg;  // No implied latches
    end
  end

  always_comb begin
    intf.dout = shift_reg;
  end
endmodule
`endif
