`ifndef SRAM_TEST_UNIT_DONE
`define SRAM_TEST_UNIT_DONE

`include "bist_if.sv"

package sram_ctl_src;
  typedef enum logic [3:0] {
    BIST,
    SCAN_CHAIN
  } sram_ctl_src_t;
endpackage

package sae_src;
  typedef enum logic [3:0] {
    INTERNAL,
    CLOCK,
    EXTERNAL
  } sae_src_t;
endpackage

interface sram_test_unit_if #(
  parameter int MAX_ADDR,
  parameter int ADDR_WIDTH = $clog2(MAX_ADDR),
  parameter int DATA_WIDTH,
  parameter int CYCLE_WIDTH = 64
) (
    input clk,
    scan_clk
);
  sram_ctl_src::sram_ctl_src_t sram_ctl_sel;
  sae_src::sae_src_t sae_sel;
  logic [3:0] sae_ctl;

  logic sram_ctl_scan_en, sram_ctl_scan_rstb, sram_ctl_scan_in;

  logic dout_scan_en, dout_scan_out;

  logic bist_en, bist_rst;
  bist_pattern::bist_pattern_t bist_pattern_sel, bist_test_pattern;
  logic bist_done, bist_fail;
  logic [ADDR_WIDTH-1:0] bist_fail_addr;
  logic [DATA_WIDTH-1:0] bist_fail_expected, bist_fail_actual, bist_fail_pattern, bist_fail_cycle;
  logic [CYCLE_WIDTH-1:0] bist_fail_cycle;

  modport sram_test_unit(
      input clk, scan_clk, sram_ctl_sel, sae_sel, sae_ctl, sram_ctl_scan_en, sram_ctl_scan_rstb, sram_ctl_scan_in, dout_scan_en, bist_en, bist_rst, bist_pattern_sel,
      output dout_scan_out, bist_test_pattern, bist_done, bist_fail
  );
endinterface

module sram_test_unit #(
  parameter int MASK_WIDTH,
  parameter int MUX_RATIO
) (
  sram_test_unit_if.sram_test_unit intf
);
  localparam int CtlWidth = ADDR_WIDTH + DATA_WIDTH + 1 + MASK_WIDTH;
  localparam int AddrOffset = DATA_WIDTH + 1 + MASK_WIDTH;
  localparam int DataOffset = 1 + MASK_WIDTH;
  localparam int WeOffset = MASK_WIDTH;

  logic [ADDR_WIDTH-1:0] addr;
  logic [DATA_WIDTH-1:0] data, dout;
  logic [MASK_WIDTH-1:0] wmask;
  logic we;

  bist_if #(
      .MAX_ADDR,
      .DATA_WIDTH,
      .MASK_WIDTH
  ) bist_if0 (
      .clk (intf.clk)
  );

  bist #(.MUX_RATIO) bist0 (.intf(bist_if0.bist));

  scan_chain_if #(.N(CtlWidth)) sram_ctl_scan_if (.scan_clk(intf.scan_clk)); // addr + data + we + wmask
  scan_chain sram_ctl_scan (.intf(sram_ctl_scan_if.scan_chain));

  scan_chain_if #(.N(DATA_WIDTH)) dout_scan_if (.scan_clk(intf.scan_clk)); // addr + data + we + wmask
  scan_chain dout_scan (.intf(sram_ctl_scan_if.scan_chain));

  if (ADDR_WIDTH == 5 && DATA_WIDTH == 32 && MASK_WIDTH == 4 && MUX_RATIO == 2) begin
    sramgen_sram_32x32m2w8_replica_v1 sram (
      .clk (intf.clk),
      .addr,
      .din (data),
      .we,
      .wmask,
      .dout
    );
  end else begin
    $error("Provided parameters do not correspond to a valid SRAM macro.");
  end

  always_comb begin
    sram_ctl_scan_if.scan_en = intf.sram_ctl_scan_en;
    sram_ctl_scan_if.scan_rstb = intf.sram_ctl_scan_rstb;
    sram_ctl_scan_if.scan_in = intf.sram_ctl_scan_in;
    sram_ctl_scan_if.rst_din = CtlWidth'(1'b0);

    dout_scan_if.scan_en = intf.dout_scan_en;
    dout_scan_if.scan_rstb = ~intf.clk;
    dout_scan_if.scan_in = 1'b0;
    dout_scan_if.rst_din = dout;
    intf.dout_scan_out = dout_scan_if.scan_out;

    bist_if0.en = intf.bist_en;
    bist_if0.rst = intf.bist_rst;
    bist_if0.pattern_sel = intf.bist_pattern_sel;
    bist_if0.dout = dout;
    intf.bist_test_pattern = bist_if0.test_pattern;
    intf.bist_done = bist_if0.done;
    intf.bist_fail = bist_if0.fail;

    case (intf.sram_ctl_sel)
      sram_ctl_src::BIST: begin
        addr = bist_if0.addr;
        data = bist_if0.data;
        wmask = bist_if0.wmask;
        we = bist_if0.we;
      end
      default: begin
        addr = sram_ctl_scan_if.dout[CtlWidth-1:AddrOffset];
        data = sram_ctl_scan_if.dout[AddrOffset-1:DataOffset];
        we = sram_ctl_scan_if.dout[DataOffset-1:WeOffset];
        wmask = sram_ctl_scan_if.dout[WeOffset-1:0];
      end
    endcase
  end

endmodule
`endif
