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
 * Receive engine
 */
module rx_engine #
(
    // DMA RAM address width
    parameter RAM_ADDR_WIDTH = 16,
    // DMA address width
    parameter DMA_ADDR_WIDTH = 64,
    // DMA length field width
    parameter DMA_LEN_WIDTH = 20,
    // DMA client length field width
    parameter DMA_CLIENT_LEN_WIDTH = 20,
    // Receive request tag field width
    parameter REQ_TAG_WIDTH = 8,
    // Descriptor request tag field width
    parameter DESC_REQ_TAG_WIDTH = 8,
    // DMA tag field width
    parameter DMA_TAG_WIDTH = 8,
    // DMA client tag field width
    parameter DMA_CLIENT_TAG_WIDTH = 8,
    // Queue request tag field width
    parameter QUEUE_REQ_TAG_WIDTH = 8,
    // Queue operation tag field width
    parameter QUEUE_OP_TAG_WIDTH = 8,
    // Queue index width
    parameter QUEUE_INDEX_WIDTH = 4,
    // Queue element pointer width
    parameter QUEUE_PTR_WIDTH = 16,
    // Completion queue index width
    parameter CPL_QUEUE_INDEX_WIDTH = 4,
    // Descriptor table size (number of in-flight operations)
    parameter DESC_TABLE_SIZE = 8,
    // Width of descriptor table field for tracking outstanding DMA operations
    parameter DESC_TABLE_DMA_OP_COUNT_WIDTH = 4,
    // Max receive packet size
    parameter MAX_RX_SIZE = 2048,
    // Receive buffer offset
    parameter RX_BUFFER_OFFSET = 0,
    // Receive buffer size
    parameter RX_BUFFER_SIZE = 16*MAX_RX_SIZE,
    // Receive buffer step size
    parameter RX_BUFFER_STEP_SIZE = 128,
    // Descriptor size (in bytes)
    parameter DESC_SIZE = 16,
    // Descriptor size (in bytes)
    parameter CPL_SIZE = 32,
    // Max number of in-flight descriptor requests
    parameter MAX_DESC_REQ = 16,
    // Width of AXI stream descriptor interfaces in bits
    parameter AXIS_DESC_DATA_WIDTH = DESC_SIZE*8,
    // AXI stream descriptor tkeep signal width (words per cycle)
    parameter AXIS_DESC_KEEP_WIDTH = AXIS_DESC_DATA_WIDTH/8,
    // Enable PTP timestamping
    parameter PTP_TS_ENABLE = 1,
    // Enable RX hashing
    parameter RX_HASH_ENABLE = 1,
    // Enable RX checksum offload
    parameter RX_CHECKSUM_ENABLE = 1
)
(
    input  wire                             clk,
    input  wire                             rst,

    /*
     * Receive request input (queue index)
     */
    input  wire [QUEUE_INDEX_WIDTH-1:0]     s_axis_rx_req_queue,
    input  wire [REQ_TAG_WIDTH-1:0]         s_axis_rx_req_tag,
    input  wire                             s_axis_rx_req_valid,
    output wire                             s_axis_rx_req_ready,

    /*
     * Receive request status output
     */
    output wire [DMA_CLIENT_LEN_WIDTH-1:0]  m_axis_rx_req_status_len,
    output wire [REQ_TAG_WIDTH-1:0]         m_axis_rx_req_status_tag,
    output wire                             m_axis_rx_req_status_valid,

    /*
     * Descriptor request output
     */
    output wire [QUEUE_INDEX_WIDTH-1:0]     m_axis_desc_req_queue,
    output wire [DESC_REQ_TAG_WIDTH-1:0]    m_axis_desc_req_tag,
    output wire                             m_axis_desc_req_valid,
    input  wire                             m_axis_desc_req_ready,

    /*
     * Descriptor request status input
     */
    input  wire [QUEUE_INDEX_WIDTH-1:0]     s_axis_desc_req_status_queue,
    input  wire [QUEUE_PTR_WIDTH-1:0]       s_axis_desc_req_status_ptr,
    input  wire [CPL_QUEUE_INDEX_WIDTH-1:0] s_axis_desc_req_status_cpl,
    input  wire [DESC_REQ_TAG_WIDTH-1:0]    s_axis_desc_req_status_tag,
    input  wire                             s_axis_desc_req_status_empty,
    input  wire                             s_axis_desc_req_status_error,
    input  wire                             s_axis_desc_req_status_valid,

    /*
     * Descriptor data input
     */
    input  wire [AXIS_DESC_DATA_WIDTH-1:0]  s_axis_desc_tdata,
    input  wire [AXIS_DESC_KEEP_WIDTH-1:0]  s_axis_desc_tkeep,
    input  wire                             s_axis_desc_tvalid,
    output wire                             s_axis_desc_tready,
    input  wire                             s_axis_desc_tlast,
    input  wire [DESC_REQ_TAG_WIDTH-1:0]    s_axis_desc_tid,
    input  wire                             s_axis_desc_tuser,

    /*
     * Completion request output
     */
    output wire [CPL_QUEUE_INDEX_WIDTH-1:0] m_axis_cpl_req_queue,
    output wire [DESC_REQ_TAG_WIDTH-1:0]    m_axis_cpl_req_tag,
    output wire [CPL_SIZE*8-1:0]            m_axis_cpl_req_data,
    output wire                             m_axis_cpl_req_valid,
    input  wire                             m_axis_cpl_req_ready,

    /*
     * Completion request status input
     */
    input  wire [DESC_REQ_TAG_WIDTH-1:0]    s_axis_cpl_req_status_tag,
    input  wire                             s_axis_cpl_req_status_full,
    input  wire                             s_axis_cpl_req_status_error,
    input  wire                             s_axis_cpl_req_status_valid,

    /*
     * DMA write descriptor output
     */
    output wire [DMA_ADDR_WIDTH-1:0]        m_axis_dma_write_desc_dma_addr,
    output wire [RAM_ADDR_WIDTH-1:0]        m_axis_dma_write_desc_ram_addr,
    output wire [DMA_LEN_WIDTH-1:0]         m_axis_dma_write_desc_len,
    output wire [DMA_TAG_WIDTH-1:0]         m_axis_dma_write_desc_tag,
    output wire                             m_axis_dma_write_desc_valid,
    input  wire                             m_axis_dma_write_desc_ready,

    /*
     * DMA write descriptor status input
     */
    input  wire [DMA_TAG_WIDTH-1:0]         s_axis_dma_write_desc_status_tag,
    input  wire [3:0]                       s_axis_dma_write_desc_status_error,
    input  wire                             s_axis_dma_write_desc_status_valid,

    /*
     * Receive descriptor output
     */
    output wire [RAM_ADDR_WIDTH-1:0]        m_axis_rx_desc_addr,
    output wire [DMA_CLIENT_LEN_WIDTH-1:0]  m_axis_rx_desc_len,
    output wire [DMA_CLIENT_TAG_WIDTH-1:0]  m_axis_rx_desc_tag,
    output wire                             m_axis_rx_desc_valid,
    input  wire                             m_axis_rx_desc_ready,

    /*
     * Receive descriptor status input
     */
    input  wire [DMA_CLIENT_LEN_WIDTH-1:0]  s_axis_rx_desc_status_len,
    input  wire [DMA_CLIENT_TAG_WIDTH-1:0]  s_axis_rx_desc_status_tag,
    input  wire                             s_axis_rx_desc_status_user,
    input  wire [3:0]                       s_axis_rx_desc_status_error,
    input  wire                             s_axis_rx_desc_status_valid,

    /*
     * Receive timestamp input
     */
    input  wire [95:0]                      s_axis_rx_ptp_ts_96,
    input  wire                             s_axis_rx_ptp_ts_valid,
    output wire                             s_axis_rx_ptp_ts_ready,

    /*
     * Receive hash input
     */
    input  wire [31:0]                      s_axis_rx_hash,
    input  wire [3:0]                       s_axis_rx_hash_type,
    input  wire                             s_axis_rx_hash_valid,
    output wire                             s_axis_rx_hash_ready,

    /*
     * Receive checksum input
     */
    input  wire [15:0]                      s_axis_rx_csum,
    input  wire                             s_axis_rx_csum_valid,
    output wire                             s_axis_rx_csum_ready,

    /*
     * Configuration
     */
    input  wire [DMA_CLIENT_LEN_WIDTH-1:0]  mtu,
    input  wire                             enable
);

initial begin
    if (RAM_ADDR_WIDTH != 19) begin $error; $finish; end
    if (DMA_ADDR_WIDTH != 64) begin $error; $finish; end
    if (DMA_LEN_WIDTH != 16) begin $error; $finish; end
    if (DMA_CLIENT_LEN_WIDTH != 16) begin $error; $finish; end
    if (REQ_TAG_WIDTH != 7) begin $error; $finish; end
    if (DESC_REQ_TAG_WIDTH != 5) begin $error; $finish; end
    if (DMA_TAG_WIDTH != 14) begin $error; $finish; end
    if (DMA_CLIENT_TAG_WIDTH != 5) begin $error; $finish; end
    if (QUEUE_INDEX_WIDTH != 8) begin $error; $finish; end
    if (QUEUE_PTR_WIDTH != 16) begin $error; $finish; end
    if (CPL_QUEUE_INDEX_WIDTH != 8) begin $error; $finish; end
    if (DESC_TABLE_SIZE != 32) begin $error; $finish; end
    if (DESC_TABLE_DMA_OP_COUNT_WIDTH != 4) begin $error; $finish; end
    if (MAX_RX_SIZE != 9214) begin $error; $finish; end
    if (RX_BUFFER_OFFSET != 0) begin $error; $finish; end
    if (RX_BUFFER_SIZE != 131072) begin $error; $finish; end
    if (RX_BUFFER_STEP_SIZE != 128) begin $error; $finish; end
    if (DESC_SIZE != 16) begin $error; $finish; end
    if (CPL_SIZE != 32) begin $error; $finish; end
    if (MAX_DESC_REQ != 16) begin $error; $finish; end
    if (AXIS_DESC_DATA_WIDTH != 128) begin $error; $finish; end
    if (AXIS_DESC_KEEP_WIDTH != 16) begin $error; $finish; end
    if (PTP_TS_ENABLE != 1) begin $error; $finish; end
    if (RX_HASH_ENABLE != 1) begin $error; $finish; end
    if (RX_CHECKSUM_ENABLE != 1) begin $error; $finish; end
end

rx_engine_inner rx_engine_inst (
    .clk(clk),
    .rst(rst),

    .s_axis_rx_req_queue(s_axis_rx_req_queue),
    .s_axis_rx_req_tag(s_axis_rx_req_tag),
    .s_axis_rx_req_valid(s_axis_rx_req_valid),
    .s_axis_rx_req_ready(s_axis_rx_req_ready),

    .m_axis_rx_req_status_len(m_axis_rx_req_status_len),
    .m_axis_rx_req_status_tag(m_axis_rx_req_status_tag),
    .m_axis_rx_req_status_valid(m_axis_rx_req_status_valid),

    .m_axis_desc_req_queue(m_axis_desc_req_queue),
    .m_axis_desc_req_tag(m_axis_desc_req_tag),
    .m_axis_desc_req_valid(m_axis_desc_req_valid),
    .m_axis_desc_req_ready(m_axis_desc_req_ready),

    .s_axis_desc_req_status_queue(s_axis_desc_req_status_queue),
    .s_axis_desc_req_status_ptr(s_axis_desc_req_status_ptr),
    .s_axis_desc_req_status_cpl(s_axis_desc_req_status_cpl),
    .s_axis_desc_req_status_tag(s_axis_desc_req_status_tag),
    .s_axis_desc_req_status_empty(s_axis_desc_req_status_empty),
    .s_axis_desc_req_status_error(s_axis_desc_req_status_error),
    .s_axis_desc_req_status_valid(s_axis_desc_req_status_valid),

    .s_axis_desc_tdata(s_axis_desc_tdata),
    .s_axis_desc_tkeep(s_axis_desc_tkeep),
    .s_axis_desc_tvalid(s_axis_desc_tvalid),
    .s_axis_desc_tready(s_axis_desc_tready),
    .s_axis_desc_tlast(s_axis_desc_tlast),
    .s_axis_desc_tid(s_axis_desc_tid),
    .s_axis_desc_tuser(s_axis_desc_tuser),

    .m_axis_cpl_req_queue(m_axis_cpl_req_queue),
    .m_axis_cpl_req_tag(m_axis_cpl_req_tag),
    .m_axis_cpl_req_data(m_axis_cpl_req_data),
    .m_axis_cpl_req_valid(m_axis_cpl_req_valid),
    .m_axis_cpl_req_ready(m_axis_cpl_req_ready),

    .s_axis_cpl_req_status_tag(s_axis_cpl_req_status_tag),
    .s_axis_cpl_req_status_full(s_axis_cpl_req_status_full),
    .s_axis_cpl_req_status_error(s_axis_cpl_req_status_error),
    .s_axis_cpl_req_status_valid(s_axis_cpl_req_status_valid),

    .m_axis_dma_write_desc_dma_addr(m_axis_dma_write_desc_dma_addr),
    .m_axis_dma_write_desc_ram_addr(m_axis_dma_write_desc_ram_addr),
    .m_axis_dma_write_desc_len(m_axis_dma_write_desc_len),
    .m_axis_dma_write_desc_tag(m_axis_dma_write_desc_tag),
    .m_axis_dma_write_desc_valid(m_axis_dma_write_desc_valid),
    .m_axis_dma_write_desc_ready(m_axis_dma_write_desc_ready),

    .s_axis_dma_write_desc_status_tag(s_axis_dma_write_desc_status_tag),
    .s_axis_dma_write_desc_status_error(s_axis_dma_write_desc_status_error),
    .s_axis_dma_write_desc_status_valid(s_axis_dma_write_desc_status_valid),

    .m_axis_rx_desc_addr(m_axis_rx_desc_addr),
    .m_axis_rx_desc_len(m_axis_rx_desc_len),
    .m_axis_rx_desc_tag(m_axis_rx_desc_tag),
    .m_axis_rx_desc_valid(m_axis_rx_desc_valid),
    .m_axis_rx_desc_ready(m_axis_rx_desc_ready),

    .s_axis_rx_desc_status_len(s_axis_rx_desc_status_len),
    .s_axis_rx_desc_status_tag(s_axis_rx_desc_status_tag),
    .s_axis_rx_desc_status_user(s_axis_rx_desc_status_user),
    .s_axis_rx_desc_status_error(s_axis_rx_desc_status_error),
    .s_axis_rx_desc_status_valid(s_axis_rx_desc_status_valid),

    .s_axis_rx_ptp_ts_96(s_axis_rx_ptp_ts_96),
    .s_axis_rx_ptp_ts_valid(s_axis_rx_ptp_ts_valid),
    .s_axis_rx_ptp_ts_ready(s_axis_rx_ptp_ts_ready),

    .s_axis_rx_hash(s_axis_rx_hash),
    .s_axis_rx_hash_type(s_axis_rx_hash_type),
    .s_axis_rx_hash_valid(s_axis_rx_hash_valid),
    .s_axis_rx_hash_ready(s_axis_rx_hash_ready),

    .s_axis_rx_csum(s_axis_rx_csum),
    .s_axis_rx_csum_valid(s_axis_rx_csum_valid),
    .s_axis_rx_csum_ready(s_axis_rx_csum_ready),

    .mtu(mtu),
    .enable(enable)
);

endmodule
