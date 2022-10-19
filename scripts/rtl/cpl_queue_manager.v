/*

Copyright 2019, The Regents of the University of California.
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

   1. Redistributions of source code must retain the above copyright notice,
      this list of conditions and the following disclaimer.

   2. Redistributions in binary form must reproduce the above copyright notice,
      this list of conditions and the following disclaimer in the documentation
      and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE REGENTS OF THE UNIVERSITY OF CALIFORNIA ''AS
IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE REGENTS OF THE UNIVERSITY OF CALIFORNIA OR
CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT
OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING
IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
OF SUCH DAMAGE.

The views and conclusions contained in the software and documentation are those
of the authors and should not be interpreted as representing official policies,
either expressed or implied, of The Regents of the University of California.

*/

// Language: Verilog 2001

`timescale 1ns / 1ps

/*
 * Completion queue manager
 */
module cpl_queue_manager #
(
    // Base address width
    parameter ADDR_WIDTH = 64,
    // Request tag field width
    parameter REQ_TAG_WIDTH = 8,
    // Number of outstanding operations
    parameter OP_TABLE_SIZE = 16,
    // Operation tag field width
    parameter OP_TAG_WIDTH = 8,
    // Queue index width (log2 of number of queues)
    parameter QUEUE_INDEX_WIDTH = 8,
    // Event index width
    parameter EVENT_WIDTH = 8,
    // Queue element pointer width (log2 of number of elements)
    parameter QUEUE_PTR_WIDTH = 16,
    // Log queue size field width
    parameter LOG_QUEUE_SIZE_WIDTH = $clog2(QUEUE_PTR_WIDTH),
    // Queue element size
    parameter CPL_SIZE = 16,
    // Pipeline stages
    parameter PIPELINE = 2,
    // Width of AXI lite data bus in bits
    parameter AXIL_DATA_WIDTH = 32,
    // Width of AXI lite address bus in bits
    parameter AXIL_ADDR_WIDTH = 16,
    // Width of AXI lite wstrb (width of data bus in words)
    parameter AXIL_STRB_WIDTH = (AXIL_DATA_WIDTH/8)
)
(
    input  wire                         clk,
    input  wire                         rst,

    /*
     * Enqueue request input
     */
    input  wire [QUEUE_INDEX_WIDTH-1:0] s_axis_enqueue_req_queue,
    input  wire [REQ_TAG_WIDTH-1:0]     s_axis_enqueue_req_tag,
    input  wire                         s_axis_enqueue_req_valid,
    output wire                         s_axis_enqueue_req_ready,

    /*
     * Enqueue response output
     */
    output wire [QUEUE_INDEX_WIDTH-1:0] m_axis_enqueue_resp_queue,
    output wire [QUEUE_PTR_WIDTH-1:0]   m_axis_enqueue_resp_ptr,
    output wire [ADDR_WIDTH-1:0]        m_axis_enqueue_resp_addr,
    output wire [EVENT_WIDTH-1:0]       m_axis_enqueue_resp_event,
    output wire [REQ_TAG_WIDTH-1:0]     m_axis_enqueue_resp_tag,
    output wire [OP_TAG_WIDTH-1:0]      m_axis_enqueue_resp_op_tag,
    output wire                         m_axis_enqueue_resp_full,
    output wire                         m_axis_enqueue_resp_error,
    output wire                         m_axis_enqueue_resp_valid,
    input  wire                         m_axis_enqueue_resp_ready,

    /*
     * Enqueue commit input
     */
    input  wire [OP_TAG_WIDTH-1:0]      s_axis_enqueue_commit_op_tag,
    input  wire                         s_axis_enqueue_commit_valid,
    output wire                         s_axis_enqueue_commit_ready,

    /*
     * Event output
     */
    output wire [EVENT_WIDTH-1:0]       m_axis_event,
    output wire [QUEUE_INDEX_WIDTH-1:0] m_axis_event_source,
    output wire                         m_axis_event_valid,

    /*
     * AXI-Lite slave interface
     */
    input  wire [AXIL_ADDR_WIDTH-1:0]   s_axil_awaddr,
    input  wire [2:0]                   s_axil_awprot,
    input  wire                         s_axil_awvalid,
    output wire                         s_axil_awready,
    input  wire [AXIL_DATA_WIDTH-1:0]   s_axil_wdata,
    input  wire [AXIL_STRB_WIDTH-1:0]   s_axil_wstrb,
    input  wire                         s_axil_wvalid,
    output wire                         s_axil_wready,
    output wire [1:0]                   s_axil_bresp,
    output wire                         s_axil_bvalid,
    input  wire                         s_axil_bready,
    input  wire [AXIL_ADDR_WIDTH-1:0]   s_axil_araddr,
    input  wire [2:0]                   s_axil_arprot,
    input  wire                         s_axil_arvalid,
    output wire                         s_axil_arready,
    output wire [AXIL_DATA_WIDTH-1:0]   s_axil_rdata,
    output wire [1:0]                   s_axil_rresp,
    output wire                         s_axil_rvalid,
    input  wire                         s_axil_rready,

    /*
     * Configuration
     */
    input  wire                         enable
);

initial begin
    if (ADDR_WIDTH != 64) begin $error; $finish; end
    if (REQ_TAG_WIDTH != 8) begin $error; $finish; end
    if (OP_TABLE_SIZE != 16) begin $error; $finish; end
    if (OP_TAG_WIDTH != 8) begin $error; $finish; end
    if (QUEUE_INDEX_WIDTH != 8) begin $error; $finish; end
    if (EVENT_WIDTH != 8) begin $error; $finish; end
    if (QUEUE_PTR_WIDTH != 16) begin $error; $finish; end
    if (CPL_SIZE != 16) begin $error; $finish; end
    if (PIPELINE != 2) begin $error; $finish; end
    if (AXIL_DATA_WIDTH != 32) begin $error; $finish; end
    if (AXIL_ADDR_WIDTH != 16) begin $error; $finish; end
    if (AXIL_STRB_WIDTH != 4) begin $error; $finish; end
end

cpl_queue_manager_inner cpl_queue_manager_inst (
    .clk(clk),
    .rst(rst),

    .request_queue(s_axis_enqueue_req_queue),
    .request_tag(s_axis_enqueue_req_tag),
    .request_valid(s_axis_enqueue_req_valid),
    .request_ready(s_axis_enqueue_req_ready),

    .response_queue(m_axis_enqueue_resp_queue),
    .response_ptr(m_axis_enqueue_resp_ptr),
    .response_addr(m_axis_enqueue_resp_addr),
    .response_event(m_axis_enqueue_resp_event),
    .response_tag(m_axis_enqueue_resp_tag),
    .response_op_tag(m_axis_enqueue_resp_op_tag),
    .response_full(m_axis_enqueue_resp_full),
    .response_error(m_axis_enqueue_resp_error),
    .response_valid(m_axis_enqueue_resp_valid),
    .response_ready(m_axis_enqueue_resp_ready),

    .commit_op_tag(s_axis_enqueue_commit_op_tag),
    .commit_valid(s_axis_enqueue_commit_valid),
    .commit_ready(s_axis_enqueue_commit_ready),

    .response_out(m_axis_event),
    .response_out_source(m_axis_event_source),
    .response_out_valid(m_axis_event_valid),

    .s_axil_awaddr(s_axil_awaddr),
    .s_axil_awprot(s_axil_awprot),
    .s_axil_awvalid(s_axil_awvalid),
    .s_axil_awready(s_axil_awready),
    .s_axil_wdata(s_axil_wdata),
    .s_axil_wstrb(s_axil_wstrb),
    .s_axil_wvalid(s_axil_wvalid),
    .s_axil_wready(s_axil_wready),
    .s_axil_bresp(s_axil_bresp),
    .s_axil_bvalid(s_axil_bvalid),
    .s_axil_bready(s_axil_bready),
    .s_axil_araddr(s_axil_araddr),
    .s_axil_arprot(s_axil_arprot),
    .s_axil_arvalid(s_axil_arvalid),
    .s_axil_arready(s_axil_arready),
    .s_axil_rdata(s_axil_rdata),
    .s_axil_rresp(s_axil_rresp),
    .s_axil_rvalid(s_axil_rvalid),
    .s_axil_rready(s_axil_rready),

    .enable(enable)
);

endmodule
