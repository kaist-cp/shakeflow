//! Constants for Corundum modules.

// Constants for `rx_hash`.
pub mod rx_hash {
    pub const DATA_WIDTH: usize = 256;
    pub const KEEP_WIDTH: usize = 32;
}

pub mod rx_hash_512 {
    pub const DATA_WIDTH: usize = 512;
    pub const KEEP_WIDTH: usize = 64;
}

// Constants for `tx_checksum`.
pub mod tx_checksum {
    pub const DATA_WIDTH: usize = 512;
    pub const KEEP_WIDTH: usize = DATA_WIDTH / 8;
    pub const ID_ENABLE: bool = false;
    pub const ID_WIDTH: usize = 8;
    pub const DEST_ENABLE: bool = false;
    pub const DEST_WIDTH: usize = 8;
    pub const USER_ENABLE: bool = true;
    pub const USER_WIDTH: usize = 1;
    pub const DATA_FIFO_DEPTH: usize = 16384;
    pub const CHECKSUM_FIFO_DEPTH: usize = 4;
}

// Constants for `event_mux`.
pub mod event_mux {
    pub const PORTS: usize = 2;
    pub const QUEUE_INDEX_WIDTH: usize = 5;
    pub const EVENT_TYPE_WIDTH: usize = 16;
    pub const EVENT_SOURCE_WIDTH: usize = 16;
}

// Constants for `cpl_op_mux`.
pub mod cpl_op_mux {
    pub const PORTS: usize = 2;
    pub const MQNIC_PORT_SELECT_WIDTH: usize = 1;
    pub const MQNIC_INTERFACE_SELECT_WIDTH: usize = 2;
    pub const MQNIC_PORT_S_REQ_TAG_WIDTH: usize = 5;
    pub const MQNIC_INTERFACE_S_REQ_TAG_WIDTH: usize = 6;
    pub const QUEUE_INDEX_WIDTH: usize = 13;
    pub const CPL_SIZE: usize = 32;
    pub const ARB_TYPE_ROUND_ROBIN: usize = 1;
    pub const ARB_LSB_HIGH_PRIORITY: usize = 1;
}

// Constants for `desc_op_mux`.
pub mod desc_op_mux {
    pub const PORTS: usize = 2;
    pub const SELECT_WIDTH: usize = 1;
    pub const QUEUE_INDEX_WIDTH: usize = 13;
    pub const QUEUE_PTR_WIDTH: usize = 16;
    pub const CPL_QUEUE_INDEX_WIDTH: usize = 13;
    pub const S_REQ_TAG_WIDTH: usize = 5;
    pub const M_REQ_TAG_WIDTH: usize = 6;
    pub const AXIS_DATA_WIDTH: usize = 128;
    pub const AXIS_KEEP_WIDTH: usize = 16;
    pub const ARB_TYPE_ROUND_ROBIN: usize = 1;
    pub const ARB_LSB_HIGH_PRIORITY: usize = 1;
}

// Constants for `desc_fetch`.
pub mod desc_fetch {
    use shakeflow::clog2;
    use static_assertions::*;

    use crate::types::dma_ram::*;

    pub const PORTS: usize = 2;
    pub const SELECT_WIDTH: usize = clog2(PORTS);
    pub const SEG_COUNT: usize = 2;
    pub const RAM_ADDR_WIDTH: usize = SEG_ADDR_WIDTH + clog2(SEG_COUNT) + clog2(SEG_BE_WIDTH);
    pub const RAM_PIPELINE: usize = 2;
    pub const DMA_ADDR_WIDTH: usize = 64;
    pub const DMA_LEN_WIDTH: usize = 16;
    pub const DMA_TAG_WIDTH: usize = 14;
    pub const REQ_TAG_WIDTH: usize = 7;
    pub const QUEUE_REQ_TAG_WIDTH: usize = 7;
    pub const QUEUE_OP_TAG_WIDTH: usize = 6;
    pub const QUEUE_INDEX_WIDTH: usize = 13;
    pub const CPL_QUEUE_INDEX_WIDTH: usize = 13;
    pub const QUEUE_PTR_WIDTH: usize = 16;
    pub const DESC_SIZE: usize = 16;
    pub const LOG_BLOCK_SIZE_WIDTH: usize = 2;
    pub const DESC_TABLE_SIZE: usize = 32;
    pub const AXIS_DATA_WIDTH: usize = DESC_SIZE * 8;
    pub const AXIS_KEEP_WIDTH: usize = AXIS_DATA_WIDTH / 8;

    pub const CL_DESC_TABLE_SIZE: usize = clog2(DESC_TABLE_SIZE);
    pub const CL_PORTS: usize = clog2(PORTS);
    pub const CL_DESC_SIZE: usize = clog2(DESC_SIZE);

    const_assert!(DMA_TAG_WIDTH >= CL_DESC_TABLE_SIZE);
    const_assert!(QUEUE_REQ_TAG_WIDTH >= CL_DESC_TABLE_SIZE);
    const_assert!(QUEUE_REQ_TAG_WIDTH >= REQ_TAG_WIDTH);
    const_assert_eq!(AXIS_KEEP_WIDTH * 8, AXIS_DATA_WIDTH);
    const_assert_eq!(1 << CL_DESC_SIZE, DESC_SIZE);
}

// Constants for `cpl_write`.
pub mod cpl_write {
    use shakeflow::clog2;
    use static_assertions::*;

    use crate::types::dma_ram::*;

    pub const PORTS: usize = 3;
    pub const SELECT_WIDTH: usize = clog2(PORTS);
    pub const SEG_COUNT: usize = 2;
    pub const RAM_ADDR_WIDTH: usize = SEG_ADDR_WIDTH + clog2(SEG_COUNT) + clog2(SEG_BE_WIDTH);
    pub const RAM_PIPELINE: usize = 2;
    pub const DMA_ADDR_WIDTH: usize = 64;
    pub const DMA_LEN_WIDTH: usize = 16;
    pub const DMA_TAG_WIDTH: usize = 14;
    pub const REQ_TAG_WIDTH: usize = 7;
    pub const QUEUE_REQ_TAG_WIDTH: usize = 7;
    pub const QUEUE_OP_TAG_WIDTH: usize = 6;
    pub const QUEUE_INDEX_WIDTH: usize = 13;
    pub const CPL_SIZE: usize = 32;
    pub const DESC_TABLE_SIZE: usize = 32;
    pub const AXIS_DATA_WIDTH: usize = CPL_SIZE * 8;
    pub const AXIS_KEEP_WIDTH: usize = CPL_SIZE;

    pub const CL_DESC_TABLE_SIZE: usize = clog2(DESC_TABLE_SIZE);
    pub const CL_PORTS: usize = clog2(PORTS);

    const_assert!(DMA_TAG_WIDTH >= CL_DESC_TABLE_SIZE);
    const_assert!(QUEUE_REQ_TAG_WIDTH >= CL_DESC_TABLE_SIZE);
}

// Constants for `queue_manager`.
pub mod queue_manager {
    use crate::fetch_queue_manager::FetchQueueManager;

    pub const PIPELINE: usize = 2;
    pub const AXIL_ADDR_WIDTH: usize = 20;
    pub const QUEUE_INDEX_WIDTH: usize = 13;
    pub const REQ_TAG_WIDTH: usize = 8;
    pub const OP_TAG_WIDTH: usize = 8;
    pub const CPL_INDEX_WIDTH: usize = 13;
    pub const OP_TABLE_SIZE: usize = 32;
    // pub const QUEUE_RAM_BE_WIDTH: usize = 16;
    pub const QUEUE_PTR_WIDTH: usize = 16;

    pub type M = FetchQueueManager<
        PIPELINE,
        AXIL_ADDR_WIDTH,
        QUEUE_INDEX_WIDTH,
        REQ_TAG_WIDTH,
        OP_TAG_WIDTH,
        CPL_INDEX_WIDTH,
        OP_TABLE_SIZE,
        QUEUE_PTR_WIDTH,
    >;
}

pub mod rx_queue_manager {
    use crate::fetch_queue_manager::FetchQueueManager;

    pub const PIPELINE: usize = 3;
    pub const AXIL_ADDR_WIDTH: usize = 20;
    pub const QUEUE_INDEX_WIDTH: usize = 13;
    pub const REQ_TAG_WIDTH: usize = 8;
    pub const OP_TAG_WIDTH: usize = 8;
    pub const CPL_INDEX_WIDTH: usize = 13;
    pub const OP_TABLE_SIZE: usize = 32;
    // pub const QUEUE_RAM_BE_WIDTH: usize = 16;
    pub const QUEUE_PTR_WIDTH: usize = 16;

    pub type M = FetchQueueManager<
        PIPELINE,
        AXIL_ADDR_WIDTH,
        QUEUE_INDEX_WIDTH,
        REQ_TAG_WIDTH,
        OP_TAG_WIDTH,
        CPL_INDEX_WIDTH,
        OP_TABLE_SIZE,
        QUEUE_PTR_WIDTH,
    >;
}

pub mod tx_queue_manager {
    use crate::fetch_queue_manager::FetchQueueManager;

    pub const PIPELINE: usize = 4;
    pub const AXIL_ADDR_WIDTH: usize = 20;
    pub const QUEUE_INDEX_WIDTH: usize = 13;
    pub const REQ_TAG_WIDTH: usize = 8;
    pub const OP_TAG_WIDTH: usize = 8;
    pub const CPL_INDEX_WIDTH: usize = 13;
    pub const OP_TABLE_SIZE: usize = 32;
    // pub const QUEUE_RAM_BE_WIDTH: usize = 16;
    pub const QUEUE_PTR_WIDTH: usize = 16;

    pub type M = FetchQueueManager<
        PIPELINE,
        AXIL_ADDR_WIDTH,
        QUEUE_INDEX_WIDTH,
        REQ_TAG_WIDTH,
        OP_TAG_WIDTH,
        CPL_INDEX_WIDTH,
        OP_TABLE_SIZE,
        QUEUE_PTR_WIDTH,
    >;
}

pub mod rx_queue_manager_bitstream {
    use crate::fetch_queue_manager::FetchQueueManager;

    pub const PIPELINE: usize = 3;
    pub const AXIL_ADDR_WIDTH: usize = 19;
    pub const QUEUE_INDEX_WIDTH: usize = 8;
    pub const REQ_TAG_WIDTH: usize = 7;
    pub const OP_TAG_WIDTH: usize = 6;
    pub const CPL_INDEX_WIDTH: usize = 8;
    pub const OP_TABLE_SIZE: usize = 32;
    // pub const QUEUE_RAM_BE_WIDTH: usize = 16;
    pub const QUEUE_PTR_WIDTH: usize = 16;

    pub type M = FetchQueueManager<
        PIPELINE,
        AXIL_ADDR_WIDTH,
        QUEUE_INDEX_WIDTH,
        REQ_TAG_WIDTH,
        OP_TAG_WIDTH,
        CPL_INDEX_WIDTH,
        OP_TABLE_SIZE,
        QUEUE_PTR_WIDTH,
    >;
}

pub mod tx_queue_manager_bitstream {
    use crate::fetch_queue_manager::FetchQueueManager;

    pub const PIPELINE: usize = 4;
    pub const AXIL_ADDR_WIDTH: usize = 20;
    pub const QUEUE_INDEX_WIDTH: usize = 13;
    pub const REQ_TAG_WIDTH: usize = 7;
    pub const OP_TAG_WIDTH: usize = 6;
    pub const CPL_INDEX_WIDTH: usize = 13;
    pub const OP_TABLE_SIZE: usize = 32;
    // pub const QUEUE_RAM_BE_WIDTH: usize = 16;
    pub const QUEUE_PTR_WIDTH: usize = 16;

    pub type M = FetchQueueManager<
        PIPELINE,
        AXIL_ADDR_WIDTH,
        QUEUE_INDEX_WIDTH,
        REQ_TAG_WIDTH,
        OP_TAG_WIDTH,
        CPL_INDEX_WIDTH,
        OP_TABLE_SIZE,
        QUEUE_PTR_WIDTH,
    >;
}

// Constants for `cpl_queue_manager`.
pub mod cpl_queue_manager {
    use crate::cpl_queue_manager::CplQueueManager;

    pub const CPL_SIZE: usize = 16;
    pub const PIPELINE: usize = 2;
    pub const REQ_TAG_WIDTH: usize = 8;
    pub const OP_TAG_WIDTH: usize = 8;
    pub const QUEUE_INDEX_WIDTH: usize = 8;
    pub const AXIL_ADDR_WIDTH: usize = 16;
    pub const EVENT_WIDTH: usize = 8;
    pub const OP_TABLE_SIZE: usize = 16;
    // pub const QUEUE_RAM_BE_WIDTH: usize = 16;
    pub const QUEUE_PTR_WIDTH: usize = 16;

    pub type M = CplQueueManager<
        CPL_SIZE,
        PIPELINE,
        REQ_TAG_WIDTH,
        OP_TAG_WIDTH,
        QUEUE_INDEX_WIDTH,
        AXIL_ADDR_WIDTH,
        EVENT_WIDTH,
        OP_TABLE_SIZE,
        QUEUE_PTR_WIDTH,
    >;
}

pub mod event_cpl_queue_manager {
    use crate::cpl_queue_manager::CplQueueManager;

    pub const CPL_SIZE: usize = 32;
    pub const PIPELINE: usize = 3;
    pub const REQ_TAG_WIDTH: usize = 8;
    pub const OP_TAG_WIDTH: usize = 8;
    pub const QUEUE_INDEX_WIDTH: usize = 13;
    pub const AXIL_ADDR_WIDTH: usize = 20;
    pub const EVENT_WIDTH: usize = 8;
    pub const OP_TABLE_SIZE: usize = 32;
    // pub const QUEUE_RAM_BE_WIDTH: usize = 16;
    pub const QUEUE_PTR_WIDTH: usize = 16;

    pub type M = CplQueueManager<
        CPL_SIZE,
        PIPELINE,
        REQ_TAG_WIDTH,
        OP_TAG_WIDTH,
        QUEUE_INDEX_WIDTH,
        AXIL_ADDR_WIDTH,
        EVENT_WIDTH,
        OP_TABLE_SIZE,
        QUEUE_PTR_WIDTH,
    >;
}

pub mod tx_cpl_queue_manager {
    use crate::cpl_queue_manager::CplQueueManager;

    pub const CPL_SIZE: usize = 32;
    pub const PIPELINE: usize = 4;
    pub const REQ_TAG_WIDTH: usize = 8;
    pub const OP_TAG_WIDTH: usize = 8;
    pub const QUEUE_INDEX_WIDTH: usize = 13;
    pub const AXIL_ADDR_WIDTH: usize = 20;
    pub const EVENT_WIDTH: usize = 8;
    pub const OP_TABLE_SIZE: usize = 32;
    // pub const QUEUE_RAM_BE_WIDTH: usize = 16;
    pub const QUEUE_PTR_WIDTH: usize = 16;

    pub type M = CplQueueManager<
        CPL_SIZE,
        PIPELINE,
        REQ_TAG_WIDTH,
        OP_TAG_WIDTH,
        QUEUE_INDEX_WIDTH,
        AXIL_ADDR_WIDTH,
        EVENT_WIDTH,
        OP_TABLE_SIZE,
        QUEUE_PTR_WIDTH,
    >;
}

pub mod rx_cpl_queue_manager {
    use crate::cpl_queue_manager::CplQueueManager;

    pub const CPL_SIZE: usize = 32;
    pub const PIPELINE: usize = 3;
    pub const REQ_TAG_WIDTH: usize = 8;
    pub const OP_TAG_WIDTH: usize = 8;
    pub const QUEUE_INDEX_WIDTH: usize = 13;
    pub const AXIL_ADDR_WIDTH: usize = 20;
    pub const EVENT_WIDTH: usize = 8;
    pub const OP_TABLE_SIZE: usize = 32;
    // pub const QUEUE_RAM_BE_WIDTH: usize = 16;
    pub const QUEUE_PTR_WIDTH: usize = 16;

    pub type M = CplQueueManager<
        CPL_SIZE,
        PIPELINE,
        REQ_TAG_WIDTH,
        OP_TAG_WIDTH,
        QUEUE_INDEX_WIDTH,
        AXIL_ADDR_WIDTH,
        EVENT_WIDTH,
        OP_TABLE_SIZE,
        QUEUE_PTR_WIDTH,
    >;
}

pub mod event_cpl_queue_manager_bitstream {
    use crate::cpl_queue_manager::CplQueueManager;

    pub const CPL_SIZE: usize = 32;
    pub const PIPELINE: usize = 3;
    pub const REQ_TAG_WIDTH: usize = 7;
    pub const OP_TAG_WIDTH: usize = 6;
    pub const QUEUE_INDEX_WIDTH: usize = 5;
    pub const AXIL_ADDR_WIDTH: usize = 19;
    pub const EVENT_WIDTH: usize = 8;
    pub const OP_TABLE_SIZE: usize = 32;
    // pub const QUEUE_RAM_BE_WIDTH: usize = 16;
    pub const QUEUE_PTR_WIDTH: usize = 16;

    pub type M = CplQueueManager<
        CPL_SIZE,
        PIPELINE,
        REQ_TAG_WIDTH,
        OP_TAG_WIDTH,
        QUEUE_INDEX_WIDTH,
        AXIL_ADDR_WIDTH,
        EVENT_WIDTH,
        OP_TABLE_SIZE,
        QUEUE_PTR_WIDTH,
    >;
}

pub mod tx_cpl_queue_manager_bitstream {
    use crate::cpl_queue_manager::CplQueueManager;

    pub const CPL_SIZE: usize = 32;
    pub const PIPELINE: usize = 4;
    pub const REQ_TAG_WIDTH: usize = 7;
    pub const OP_TAG_WIDTH: usize = 6;
    pub const QUEUE_INDEX_WIDTH: usize = 13;
    pub const AXIL_ADDR_WIDTH: usize = 20;
    pub const EVENT_WIDTH: usize = 5;
    pub const OP_TABLE_SIZE: usize = 32;
    // pub const QUEUE_RAM_BE_WIDTH: usize = 16;
    pub const QUEUE_PTR_WIDTH: usize = 16;

    pub type M = CplQueueManager<
        CPL_SIZE,
        PIPELINE,
        REQ_TAG_WIDTH,
        OP_TAG_WIDTH,
        QUEUE_INDEX_WIDTH,
        AXIL_ADDR_WIDTH,
        EVENT_WIDTH,
        OP_TABLE_SIZE,
        QUEUE_PTR_WIDTH,
    >;
}

pub mod rx_cpl_queue_manager_bitstream {
    use crate::cpl_queue_manager::CplQueueManager;

    pub const CPL_SIZE: usize = 32;
    pub const PIPELINE: usize = 3;
    pub const REQ_TAG_WIDTH: usize = 7;
    pub const OP_TAG_WIDTH: usize = 6;
    pub const QUEUE_INDEX_WIDTH: usize = 8;
    pub const AXIL_ADDR_WIDTH: usize = 19;
    pub const EVENT_WIDTH: usize = 5;
    pub const OP_TABLE_SIZE: usize = 32;
    // pub const QUEUE_RAM_BE_WIDTH: usize = 16;
    pub const QUEUE_PTR_WIDTH: usize = 16;

    pub type M = CplQueueManager<
        CPL_SIZE,
        PIPELINE,
        REQ_TAG_WIDTH,
        OP_TAG_WIDTH,
        QUEUE_INDEX_WIDTH,
        AXIL_ADDR_WIDTH,
        EVENT_WIDTH,
        OP_TABLE_SIZE,
        QUEUE_PTR_WIDTH,
    >;
}

// Constants for `tx_engine`.
pub mod tx_engine {
    use shakeflow::clog2;
    use static_assertions::*;

    pub const RAM_ADDR_WIDTH: usize = 19;
    // pub const DMA_ADDR_WIDTH: usize = 64;
    // pub const DMA_LEN_WIDTH: usize = 16;
    pub const DMA_CLIENT_LEN_WIDTH: usize = 16;
    pub const REQ_TAG_WIDTH: usize = 7;
    pub const DESC_REQ_TAG_WIDTH: usize = 5;
    // pub const DMA_TAG_WIDTH: usize = 14;
    pub const DMA_CLIENT_TAG_WIDTH: usize = 5;
    pub const QUEUE_INDEX_WIDTH: usize = 13;
    pub const QUEUE_PTR_WIDTH: usize = 16;
    pub const CPL_QUEUE_INDEX_WIDTH: usize = 13;
    pub const DESC_TABLE_SIZE: usize = 32;
    pub const DESC_TABLE_DMA_OP_COUNT_WIDTH: usize = 4;
    pub const MAX_TX_SIZE: usize = 9214;
    // pub const TX_BUFFER_OFFSET: usize = 0;
    pub const TX_BUFFER_SIZE: usize = 131072;
    pub const TX_BUFFER_STEP_SIZE: usize = 128;
    pub const DESC_SIZE: usize = 16;
    pub const CPL_SIZE: usize = 32;
    pub const MAX_DESC_REQ: usize = 16;
    pub const AXIS_DESC_DATA_WIDTH: usize = DESC_SIZE * 8;
    pub const AXIS_DESC_KEEP_WIDTH: usize = AXIS_DESC_DATA_WIDTH / 8;
    // pub const PTP_TS_ENABLE: usize = 1;
    // pub const TX_CHECKSUM_ENABLE: usize = 1;
    pub const CPL_DATA_SIZE: usize = CPL_SIZE * 8;
    pub const CL_DESC_TABLE_SIZE: usize = clog2(DESC_TABLE_SIZE);

    pub const CL_TX_BUFFER_SIZE: usize = clog2(TX_BUFFER_SIZE);
    pub const TX_BUFFER_PTR_MASK: usize = (1 << CL_TX_BUFFER_SIZE) - 1;
    pub const TX_BUFFER_PTR_MASK_LOWER: usize = (1 << clog2(TX_BUFFER_STEP_SIZE)) - 1;

    // const_assert!(DMA_TAG_WIDTH >= CL_DESC_TABLE_SIZE);
    const_assert!(DMA_CLIENT_TAG_WIDTH >= CL_DESC_TABLE_SIZE);
    const_assert!(DESC_REQ_TAG_WIDTH >= CL_DESC_TABLE_SIZE);
}

// Constants for `rx_engine`.
pub mod rx_engine {
    use shakeflow::clog2;
    use static_assertions::*;

    pub const RAM_ADDR_WIDTH: usize = 19;
    // pub const DMA_ADDR_WIDTH: usize = 64;
    // pub const DMA_LEN_WIDTH: usize = 16;
    pub const DMA_CLIENT_LEN_WIDTH: usize = 16;
    pub const REQ_TAG_WIDTH: usize = 7;
    pub const DESC_REQ_TAG_WIDTH: usize = 5;
    // pub const DMA_TAG_WIDTH: usize = 14;
    pub const DMA_CLIENT_TAG_WIDTH: usize = 5;
    pub const QUEUE_INDEX_WIDTH: usize = 8;
    pub const QUEUE_PTR_WIDTH: usize = 16;
    pub const CPL_QUEUE_INDEX_WIDTH: usize = 8;
    pub const DESC_TABLE_SIZE: usize = 32;
    pub const DESC_TABLE_DMA_OP_COUNT_WIDTH: usize = 4;
    pub const MAX_RX_SIZE: usize = 9214;
    // pub const RX_BUFFER_OFFSET: usize = 0;
    pub const RX_BUFFER_SIZE: usize = 131072;
    pub const RX_BUFFER_STEP_SIZE: usize = 128;
    pub const DESC_SIZE: usize = 16;
    pub const CPL_SIZE: usize = 32;
    pub const MAX_DESC_REQ: usize = 16;
    pub const AXIS_DESC_DATA_WIDTH: usize = DESC_SIZE * 8;
    pub const AXIS_DESC_KEEP_WIDTH: usize = AXIS_DESC_DATA_WIDTH / 8;
    // pub const PTP_TS_ENABLE: usize = 1;
    // pub const RX_HASH_ENABLE: usize = 1;
    // pub const RX_CHECKSUM_ENABLE: usize = 1;

    pub const CPL_DATA_SIZE: usize = CPL_SIZE * 8;
    pub const CL_DESC_TABLE_SIZE: usize = clog2(DESC_TABLE_SIZE);

    pub const CL_RX_BUFFER_SIZE: usize = clog2(RX_BUFFER_SIZE);
    pub const RX_BUFFER_PTR_MASK: usize = (1 << CL_RX_BUFFER_SIZE) - 1;
    pub const RX_BUFFER_PTR_MASK_LOWER: usize = (1 << clog2(RX_BUFFER_STEP_SIZE)) - 1;

    // const_assert!(DMA_TAG_WIDTH >= CL_DESC_TABLE_SIZE);
    const_assert!(DMA_CLIENT_TAG_WIDTH >= CL_DESC_TABLE_SIZE);
    const_assert!(DESC_REQ_TAG_WIDTH >= CL_DESC_TABLE_SIZE);
}

// Constants for `tx_scheduler_rr`.
pub mod tx_scheduler_rr {
    use static_assertions::*;

    pub const PIPELINE: usize = 4;
    pub const QUEUE_INDEX_WIDTH: usize = 13;
    pub const QUEUE_RAM_WIDTH: usize = 16;
    pub const OP_TABLE_SIZE: usize = 32;
    pub const QUEUE_COUNT: usize = 1 << 13;
    pub const AXIL_ADDR_WIDTH: usize = 20;
    pub const LEN_WIDTH: usize = 16;
    pub const REQ_TAG_WIDTH: usize = 7;
    // pub const OP_TABLE_SIZE: usize = 32;
    // pub const SCHED_CTRL_ENABLE: usize = 0;
    // pub const QUEUE_COUNT: usize = 1 << QUEUE_INDEX_WIDTH;
    // pub const QUEUE_RAM_BE_WIDTH: usize = 2;
    // pub const QUEUE_RAM_WIDTH: usize = QUEUE_RAM_BE_WIDTH * 8;

    // const_assert!(REQ_TAG_WIDTH >= clog2(OP_TABLE_SIZE));
    const_assert!(AXIL_ADDR_WIDTH >= QUEUE_INDEX_WIDTH + 5);
}
