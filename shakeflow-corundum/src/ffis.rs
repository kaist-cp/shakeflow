//! FFIs.

use shakeflow::*;

use super::cpl_write::*;
use super::desc_fetch::*;
use super::types::dma_ram::*;

impl_custom_inst! {
    DmaPsdpramI <> & <const IN_SEG_COUNT: usize>,
    DmaPsdpramO <> & <const OUT_SEG_COUNT: usize>,
    dma_psdpram,
    <SIZE, SEG_COUNT, SEG_DATA_WIDTH, SEG_ADDR_WIDTH, SEG_BE_WIDTH, PIPELINE>,
    true,
}

impl_custom_inst! {
    DmaClientAxisSinkI <> & <>,
    DmaClientAxisSinkO <> & <>,
    dma_client_axis_sink,
    <SEG_COUNT, SEG_DATA_WIDTH, SEG_ADDR_WIDTH, SEG_BE_WIDTH, RAM_ADDR_WIDTH, AXIS_DATA_WIDTH, AXIS_KEEP_ENABLE, AXIS_KEEP_WIDTH, AXIS_LAST_ENABLE, AXIS_ID_ENABLE, AXIS_DEST_ENABLE, AXIS_USER_ENABLE, AXIS_USER_WIDTH, LEN_WIDTH, TAG_WIDTH>,
    true,
}

impl_custom_inst! {
    DmaClientAxisSourceI <> & <>,
    DmaClientAxisSourceO <> & <>,
    dma_client_axis_source,
    <SEG_COUNT, SEG_DATA_WIDTH, SEG_ADDR_WIDTH, SEG_BE_WIDTH, RAM_ADDR_WIDTH, AXIS_DATA_WIDTH, AXIS_KEEP_ENABLE, AXIS_KEEP_WIDTH, AXIS_LAST_ENABLE, AXIS_ID_ENABLE, AXIS_ID_WIDTH, AXIS_DEST_ENABLE, AXIS_USER_ENABLE, AXIS_USER_WIDTH, LEN_WIDTH, TAG_WIDTH>,
    true,
}
