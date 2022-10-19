#![allow(clippy::needless_lifetimes)]
#![allow(clippy::type_complexity)]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#![feature(type_changing_struct_update)]
#![feature(adt_const_params)]

use std::path::Path;

use shakeflow::{Package, PackageError};

mod cmac_pad;
mod constants;
mod cpl_op_mux;
mod cpl_queue_manager;
mod cpl_write;
mod desc_fetch;
mod desc_op_mux;
mod event_mux;
mod fetch_queue_manager;
mod ffis;
mod queue_manager;
mod rx_checksum;
mod rx_engine;
mod rx_hash;
mod tx_checksum;
mod tx_engine;
mod tx_scheduler_rr;
mod types;

fn main() -> Result<(), PackageError> {
    let mut package = Package::default();
    package.add(cmac_pad::m());
    package.add(rx_checksum::m("rx_checksum"));
    package.add(event_mux::m());
    package.add(rx_hash::m::<{ constants::rx_hash::DATA_WIDTH }, { constants::rx_hash::KEEP_WIDTH }>("rx_hash"));
    package.add(tx_checksum::m("tx_checksum"));
    package.add(cpl_op_mux::m::<
        { constants::cpl_op_mux::PORTS },
        { constants::cpl_op_mux::MQNIC_PORT_SELECT_WIDTH },
        { constants::cpl_op_mux::MQNIC_PORT_S_REQ_TAG_WIDTH },
    >("cpl_op_mux_mqnic_port"));
    package.add(cpl_op_mux::m::<
        { constants::cpl_op_mux::PORTS },
        { constants::cpl_op_mux::MQNIC_INTERFACE_SELECT_WIDTH },
        { constants::cpl_op_mux::MQNIC_INTERFACE_S_REQ_TAG_WIDTH },
    >("cpl_op_mux_mqnic_interface"));
    package.add(desc_fetch::m());
    package.add(cpl_write::m());
    package.add(queue_manager::m::<
        { constants::queue_manager::PIPELINE },
        { constants::queue_manager::AXIL_ADDR_WIDTH },
        { constants::queue_manager::QUEUE_INDEX_WIDTH },
        { constants::queue_manager::REQ_TAG_WIDTH },
        { constants::queue_manager::OP_TAG_WIDTH },
        { constants::queue_manager::QUEUE_PTR_WIDTH },
        constants::queue_manager::M,
    >("queue_manager"));
    package.add(queue_manager::m::<
        { constants::cpl_queue_manager::PIPELINE },
        { constants::cpl_queue_manager::AXIL_ADDR_WIDTH },
        { constants::cpl_queue_manager::QUEUE_INDEX_WIDTH },
        { constants::cpl_queue_manager::REQ_TAG_WIDTH },
        { constants::cpl_queue_manager::OP_TAG_WIDTH },
        { constants::cpl_queue_manager::QUEUE_PTR_WIDTH },
        constants::cpl_queue_manager::M,
    >("cpl_queue_manager"));
    package.add(desc_op_mux::m());
    package.add(tx_engine::m());
    package.add(rx_engine::m());
    package.add(tx_scheduler_rr::m::<
        { constants::tx_scheduler_rr::PIPELINE },
        { constants::tx_scheduler_rr::QUEUE_INDEX_WIDTH },
        { constants::tx_scheduler_rr::QUEUE_RAM_WIDTH },
        { constants::tx_scheduler_rr::OP_TABLE_SIZE },
        { constants::tx_scheduler_rr::QUEUE_COUNT },
    >());

    // TODO: Some module parameters differ between the tests for its own module and
    // fpga_core. We currently don't support parameters, so we're destined to duplicate the logic
    // and generate modules with different parameters for testing both. In the future, we need
    // to support module parameters and merge that modules.
    package.add(rx_checksum::m("rx_checksum_512"));
    package.add(rx_hash::m::<{ constants::rx_hash_512::DATA_WIDTH }, { constants::rx_hash_512::KEEP_WIDTH }>(
        "rx_hash_512",
    ));
    package.add(tx_checksum::m("tx_checksum_512"));
    package.add(queue_manager::m::<
        { constants::rx_queue_manager::PIPELINE },
        { constants::rx_queue_manager::AXIL_ADDR_WIDTH },
        { constants::rx_queue_manager::QUEUE_INDEX_WIDTH },
        { constants::rx_queue_manager::REQ_TAG_WIDTH },
        { constants::rx_queue_manager::OP_TAG_WIDTH },
        { constants::rx_queue_manager::QUEUE_PTR_WIDTH },
        constants::rx_queue_manager::M,
    >("rx_queue_manager"));
    package.add(queue_manager::m::<
        { constants::tx_queue_manager::PIPELINE },
        { constants::tx_queue_manager::AXIL_ADDR_WIDTH },
        { constants::tx_queue_manager::QUEUE_INDEX_WIDTH },
        { constants::tx_queue_manager::REQ_TAG_WIDTH },
        { constants::tx_queue_manager::OP_TAG_WIDTH },
        { constants::tx_queue_manager::QUEUE_PTR_WIDTH },
        constants::tx_queue_manager::M,
    >("tx_queue_manager"));
    package.add(queue_manager::m::<
        { constants::rx_queue_manager_bitstream::PIPELINE },
        { constants::rx_queue_manager_bitstream::AXIL_ADDR_WIDTH },
        { constants::rx_queue_manager_bitstream::QUEUE_INDEX_WIDTH },
        { constants::rx_queue_manager_bitstream::REQ_TAG_WIDTH },
        { constants::rx_queue_manager_bitstream::OP_TAG_WIDTH },
        { constants::rx_queue_manager_bitstream::QUEUE_PTR_WIDTH },
        constants::rx_queue_manager_bitstream::M,
    >("rx_queue_manager_bitstream"));
    package.add(queue_manager::m::<
        { constants::tx_queue_manager_bitstream::PIPELINE },
        { constants::tx_queue_manager_bitstream::AXIL_ADDR_WIDTH },
        { constants::tx_queue_manager_bitstream::QUEUE_INDEX_WIDTH },
        { constants::tx_queue_manager_bitstream::REQ_TAG_WIDTH },
        { constants::tx_queue_manager_bitstream::OP_TAG_WIDTH },
        { constants::tx_queue_manager_bitstream::QUEUE_PTR_WIDTH },
        constants::tx_queue_manager_bitstream::M,
    >("tx_queue_manager_bitstream"));
    package.add(queue_manager::m::<
        { constants::event_cpl_queue_manager::PIPELINE },
        { constants::event_cpl_queue_manager::AXIL_ADDR_WIDTH },
        { constants::event_cpl_queue_manager::QUEUE_INDEX_WIDTH },
        { constants::event_cpl_queue_manager::REQ_TAG_WIDTH },
        { constants::event_cpl_queue_manager::OP_TAG_WIDTH },
        { constants::event_cpl_queue_manager::QUEUE_PTR_WIDTH },
        constants::event_cpl_queue_manager::M,
    >("event_cpl_queue_manager"));
    package.add(queue_manager::m::<
        { constants::tx_cpl_queue_manager::PIPELINE },
        { constants::tx_cpl_queue_manager::AXIL_ADDR_WIDTH },
        { constants::tx_cpl_queue_manager::QUEUE_INDEX_WIDTH },
        { constants::tx_cpl_queue_manager::REQ_TAG_WIDTH },
        { constants::tx_cpl_queue_manager::OP_TAG_WIDTH },
        { constants::tx_cpl_queue_manager::QUEUE_PTR_WIDTH },
        constants::tx_cpl_queue_manager::M,
    >("tx_cpl_queue_manager"));
    package.add(queue_manager::m::<
        { constants::rx_cpl_queue_manager::PIPELINE },
        { constants::rx_cpl_queue_manager::AXIL_ADDR_WIDTH },
        { constants::rx_cpl_queue_manager::QUEUE_INDEX_WIDTH },
        { constants::rx_cpl_queue_manager::REQ_TAG_WIDTH },
        { constants::rx_cpl_queue_manager::OP_TAG_WIDTH },
        { constants::rx_cpl_queue_manager::QUEUE_PTR_WIDTH },
        constants::rx_cpl_queue_manager::M,
    >("rx_cpl_queue_manager"));
    package.add(queue_manager::m::<
        { constants::event_cpl_queue_manager_bitstream::PIPELINE },
        { constants::event_cpl_queue_manager_bitstream::AXIL_ADDR_WIDTH },
        { constants::event_cpl_queue_manager_bitstream::QUEUE_INDEX_WIDTH },
        { constants::event_cpl_queue_manager_bitstream::REQ_TAG_WIDTH },
        { constants::event_cpl_queue_manager_bitstream::OP_TAG_WIDTH },
        { constants::event_cpl_queue_manager_bitstream::QUEUE_PTR_WIDTH },
        constants::event_cpl_queue_manager_bitstream::M,
    >("event_cpl_queue_manager_bitstream"));
    package.add(queue_manager::m::<
        { constants::tx_cpl_queue_manager_bitstream::PIPELINE },
        { constants::tx_cpl_queue_manager_bitstream::AXIL_ADDR_WIDTH },
        { constants::tx_cpl_queue_manager_bitstream::QUEUE_INDEX_WIDTH },
        { constants::tx_cpl_queue_manager_bitstream::REQ_TAG_WIDTH },
        { constants::tx_cpl_queue_manager_bitstream::OP_TAG_WIDTH },
        { constants::tx_cpl_queue_manager_bitstream::QUEUE_PTR_WIDTH },
        constants::tx_cpl_queue_manager_bitstream::M,
    >("tx_cpl_queue_manager_bitstream"));
    package.add(queue_manager::m::<
        { constants::rx_cpl_queue_manager_bitstream::PIPELINE },
        { constants::rx_cpl_queue_manager_bitstream::AXIL_ADDR_WIDTH },
        { constants::rx_cpl_queue_manager_bitstream::QUEUE_INDEX_WIDTH },
        { constants::rx_cpl_queue_manager_bitstream::REQ_TAG_WIDTH },
        { constants::rx_cpl_queue_manager_bitstream::OP_TAG_WIDTH },
        { constants::rx_cpl_queue_manager_bitstream::QUEUE_PTR_WIDTH },
        constants::rx_cpl_queue_manager_bitstream::M,
    >("rx_cpl_queue_manager_bitstream"));

    package.gen_vir(Path::new("./build"))
}
