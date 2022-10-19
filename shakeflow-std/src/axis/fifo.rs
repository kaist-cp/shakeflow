//! FIFO.

use super::*;

/// Axis FIFO.
pub trait AxisFifoExt: Interface {
    /// Output interface type
    type Out: Interface;

    /// Axis FIFO.
    fn axis_fifo<
        Depth: Num,
        const DATA_WIDTH: usize,
        const KEEP_ENABLE: bool,
        const KEEP_WIDTH: usize,
        const LAST_ENABLE: bool,
        const ID_ENABLE: bool,
        const ID_WIDTH: usize,
        const DEST_ENABLE: bool,
        const DEST_WIDTH: usize,
        const USER_ENABLE: bool,
        const USER_WIDTH: usize,
        const FRAME_FIFO: usize,
    >(
        self, k: &mut CompositeModuleContext, inst_name: &str,
    ) -> Self::Out;
}

impl<V: Signal, const P: Protocol> AxisFifoExt for AxisVrChannel<V, P> {
    type Out = AxisVrChannel<V>;

    fn axis_fifo<
        Depth: Num,
        const DATA_WIDTH: usize,
        const KEEP_ENABLE: bool,
        const KEEP_WIDTH: usize,
        const LAST_ENABLE: bool,
        const ID_ENABLE: bool,
        const ID_WIDTH: usize,
        const DEST_ENABLE: bool,
        const DEST_WIDTH: usize,
        const USER_ENABLE: bool,
        const USER_WIDTH: usize,
        const FRAME_FIFO: usize,
    >(
        self, k: &mut CompositeModuleContext, inst_name: &str,
    ) -> AxisVrChannel<V> {
        let mut params = Vec::new();

        params.push(("DEPTH", Depth::WIDTH));
        params.push(("DATA_WIDTH", DATA_WIDTH));
        params.push(("KEEP_ENABLE", usize::from(KEEP_ENABLE)));
        params.push(("KEEP_WIDTH", if KEEP_WIDTH > 0 { KEEP_WIDTH } else { DATA_WIDTH / 8 }));
        params.push(("LAST_ENABLE", usize::from(LAST_ENABLE)));
        params.push(("ID_ENABLE", usize::from(ID_ENABLE)));
        params.push(("ID_WIDTH", if ID_WIDTH > 0 { ID_WIDTH } else { 8 }));
        params.push(("DEST_ENABLE", usize::from(DEST_ENABLE)));
        params.push(("DEST_WIDTH", if DEST_WIDTH > 0 { DEST_WIDTH } else { 8 }));
        params.push(("USER_ENABLE", usize::from(USER_ENABLE)));
        params.push(("USER_WIDTH", if USER_WIDTH > 0 { USER_WIDTH } else { 1 }));
        params.push(("FRAME_FIFO", FRAME_FIFO));

        self.module_inst::<AxisVrChannel<V>>(k, "axis_fifo", inst_name, params, true, Some("s_axis"), Some("m_axis"))
    }
}
