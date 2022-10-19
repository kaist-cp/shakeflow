use shakeflow::*;
use shakeflow_std::axis::{AxisChannel, Keep};

pub const QSFP28_DATA_WIDTH: usize = 512;
pub const QSFP28_KEEP_WIDTH: usize = QSFP28_DATA_WIDTH / 8;
pub const QSFP28_USER_WIDTH: usize = 1;

/// Payload of QSFP28 channel.
#[derive(Debug, Clone, Signal)]
pub struct Qsfp28Payload<const QSFP28_DATA_WIDTH: usize, const QSFP28_KEEP_WIDTH: usize> {
    #[member(name = "")]
    data: Keep<U<QSFP28_DATA_WIDTH>, U<QSFP28_KEEP_WIDTH>>,
    tuser: Bits<U<QSFP28_USER_WIDTH>>,
}

pub type Qsfp28Channel = AxisChannel<Qsfp28Payload<QSFP28_DATA_WIDTH, QSFP28_KEEP_WIDTH>>;
