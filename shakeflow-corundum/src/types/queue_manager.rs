//! Types used for queue manager.

use shakeflow::*;
use shakeflow_std::*;
use static_assertions::*;

use super::axil::*;
use super::request::*;

/// Base address width
pub const ADDR_WIDTH: usize = 64;

/// Log desc block size field width
pub const LOG_BLOCK_SIZE_WIDTH: usize = 2;

/// Queue element size
pub const DESC_SIZE: usize = 16;

pub const QUEUE_RAM_BE_WIDTH: usize = 16;

pub const QUEUE_RAM_WIDTH: usize = QUEUE_RAM_BE_WIDTH * 8;

pub const CL_DESC_SIZE: usize = clog2(DESC_SIZE);

// Error: OP_TAG_WIDTH insufficient for OP_TABLE_SIZE (instance %m)
// const_assert!(OP_TAG_WIDTH >= CL_OP_TABLE_SIZE);

// Error: AXI lite address width too narrow (instance %m)
// const_assert!(AXIL_ADDR_WIDTH >= QUEUE_INDEX_WIDTH + 5);

// Error: Descriptor size must be even power of two (instance %m)
const_assert!(DESC_SIZE.is_power_of_two());

#[derive(Debug, Clone, Signal)]
pub struct CommitReq<const OP_TAG_WIDTH: usize> {
    op_tag: Bits<U<OP_TAG_WIDTH>>,
}

#[derive(Debug, Clone, Signal)]
pub struct Commit<const QUEUE_INDEX_WIDTH: usize, const QUEUE_PTR_WIDTH: usize> {
    op_table_queue: Bits<U<QUEUE_INDEX_WIDTH>>,
    op_table_queue_ptr: Bits<U<QUEUE_PTR_WIDTH>>,
}

#[derive(Debug, Clone, Signal)]
pub struct DeqRes<
    const QUEUE_INDEX_WIDTH: usize,
    const CPL_INDEX_WIDTH: usize,
    const REQ_TAG_WIDTH: usize,
    const OP_TAG_WIDTH: usize,
    const QUEUE_PTR_WIDTH: usize,
> {
    queue: Bits<U<QUEUE_INDEX_WIDTH>>,
    ptr: Bits<U<QUEUE_PTR_WIDTH>>,
    addr: Bits<U<ADDR_WIDTH>>,
    block_size: Bits<U<LOG_BLOCK_SIZE_WIDTH>>,
    cpl: Bits<U<CPL_INDEX_WIDTH>>,
    tag: Bits<U<REQ_TAG_WIDTH>>,
    op_tag: Bits<U<OP_TAG_WIDTH>>,
    empty: bool,
    error: bool,
}

#[derive(Debug, Clone, Signal)]
pub struct EnqRes<
    const QUEUE_INDEX_WIDTH: usize,
    const EVENT_WIDTH: usize,
    const REQ_TAG_WIDTH: usize,
    const OP_TAG_WIDTH: usize,
    const QUEUE_PTR_WIDTH: usize,
> {
    queue: Bits<U<QUEUE_INDEX_WIDTH>>,
    ptr: Bits<U<QUEUE_PTR_WIDTH>>,
    addr: Bits<U<ADDR_WIDTH>>,
    event: Bits<U<EVENT_WIDTH>>,
    tag: Bits<U<REQ_TAG_WIDTH>>,
    op_tag: Bits<U<OP_TAG_WIDTH>>,
    full: bool,
    error: bool,
}

#[derive(Debug, Clone, Signal)]
pub struct Doorbell<const QUEUE_INDEX_WIDTH: usize> {
    queue: Bits<U<QUEUE_INDEX_WIDTH>>,
}

#[derive(Debug, Clone, Signal)]
pub struct Event<const EVENT_WIDTH: usize, const QUEUE_INDEX_WIDTH: usize> {
    #[member(name = "")]
    event: Bits<U<EVENT_WIDTH>>,
    source: Bits<U<QUEUE_INDEX_WIDTH>>,
}

#[derive(Debug, Default, Clone, Copy, Signal)]
pub struct Selector {
    /// AXIL write
    write: bool,

    /// AXIL read
    read: bool,

    /// Commit
    commit: bool,

    /// Request
    req: bool,
}

impl Selector {
    /// Creates new selector expr from bits.
    ///
    /// # Note
    ///
    /// Order of bits should follow the order of selector fields.
    pub fn from_bits<'id>(selector: Expr<'id, Bits<U<4>>>) -> Expr<'id, Self> {
        SelectorProj { write: selector[0], read: selector[1], commit: selector[2], req: selector[3] }.into()
    }

    pub fn is_active<'id>(selector: Expr<'id, Self>) -> Expr<'id, bool> {
        selector.read | selector.write | selector.commit | selector.req
    }

    pub fn bitor<'id>(lhs: Expr<'id, Self>, rhs: Expr<'id, Self>) -> Expr<'id, Self> {
        SelectorProj {
            write: lhs.write | rhs.write,
            read: lhs.read | rhs.read,
            commit: lhs.commit | rhs.commit,
            req: lhs.req | rhs.req,
        }
        .into()
    }
}

#[derive(Debug, Clone, Signal)]
pub struct OpTableStart<const QUEUE_INDEX_WIDTH: usize, const QUEUE_PTR_WIDTH: usize> {
    queue: Bits<U<QUEUE_INDEX_WIDTH>>,
    queue_ptr: Bits<U<QUEUE_PTR_WIDTH>>,
}

#[derive(Debug, Clone, Signal)]
pub struct OpTableEntry<const QUEUE_INDEX_WIDTH: usize, const QUEUE_PTR_WIDTH: usize> {
    active: bool,
    commit: bool,
    queue: Bits<U<QUEUE_INDEX_WIDTH>>,
    ptr: Bits<U<QUEUE_PTR_WIDTH>>,
}

pub fn update_op_table<
    'id,
    Pipeline: Num,
    QueueCount: Num,
    const OP_TABLE_SIZE: usize,
    const QUEUE_INDEX_WIDTH: usize,
    const QUEUE_PTR_WIDTH: usize,
>(
    state: Expr<'id, OpState<Pipeline, QueueCount, OP_TABLE_SIZE, QUEUE_INDEX_WIDTH, QUEUE_PTR_WIDTH>>,
    op_table_start: Expr<'id, Valid<OpTableStart<QUEUE_INDEX_WIDTH, QUEUE_PTR_WIDTH>>>,
    op_table_finish: Expr<'id, Valid<()>>, op_table_commit: Expr<'id, Valid<Bits<Log2<U<OP_TABLE_SIZE>>>>>,
) -> Expr<'id, OpState<Pipeline, QueueCount, OP_TABLE_SIZE, QUEUE_INDEX_WIDTH, QUEUE_PTR_WIDTH>> {
    let mut state = *state;
    let mut op_table = *state.op_table;

    // op_table_start_en
    let op_table_start_en = op_table_start.valid;
    op_table.active = if_then_set! { op_table.active, op_table_start_en, state.op_table_start_ptr, Expr::from(true) };
    op_table.commit = if_then_set! { op_table.commit, op_table_start_en, state.op_table_start_ptr, Expr::from(false) };
    op_table.queue = if_then_set_var_arr! { op_table.queue, op_table_start_en, state.op_table_start_ptr, op_table_start.inner.queue };
    op_table.ptr = if_then_set_var_arr! { op_table.ptr, op_table_start_en, state.op_table_start_ptr, op_table_start.inner.queue_ptr };

    state.op_table_start_ptr =
        op_table_start_en.cond((state.op_table_start_ptr + 1.into()).resize(), state.op_table_start_ptr);

    // op_table_finish_en
    let op_table_finish_en = op_table_finish.valid;
    op_table.active =
        if_then_set! { op_table.active, op_table_finish_en, state.op_table_finish_ptr, Expr::from(false) };

    state.op_table_finish_ptr =
        (!op_table_finish_en).cond(state.op_table_finish_ptr, (state.op_table_finish_ptr + 1.into()).resize());

    // op_table_commit_en
    let op_table_commit_en = op_table_commit.valid;
    op_table.commit = if_then_set! { op_table.commit, op_table_commit_en, op_table_commit.inner, Expr::from(true) };

    state.set_op_table(op_table.into())
}

#[derive(Debug, Clone, Signal)]
pub struct OpState<
    Pipeline: Num,
    QueueCount: Num,
    const OP_TABLE_SIZE: usize,
    const QUEUE_INDEX_WIDTH: usize,
    const QUEUE_PTR_WIDTH: usize,
> {
    queue_ram: VarArray<Bits<U<QUEUE_RAM_WIDTH>>, QueueCount>,
    queue_ram_read_data: VarArray<Bits<U<QUEUE_RAM_WIDTH>>, Pipeline>,
    op_table: OpTableEntryVarArr<QUEUE_INDEX_WIDTH, QUEUE_PTR_WIDTH, OP_TABLE_SIZE>,
    op_table_start_ptr: Bits<Log2<U<OP_TABLE_SIZE>>>,
    op_table_finish_ptr: Bits<Log2<U<OP_TABLE_SIZE>>>,
}

impl<
        Pipeline: Num,
        QueueCount: Num,
        const OP_TABLE_SIZE: usize,
        const QUEUE_INDEX_WIDTH: usize,
        const QUEUE_PTR_WIDTH: usize,
    > OpState<Pipeline, QueueCount, OP_TABLE_SIZE, QUEUE_INDEX_WIDTH, QUEUE_PTR_WIDTH>
{
    pub fn new_expr() -> Expr<'static, Self> {
        OpStateProj {
            queue_ram: Expr::x(),
            queue_ram_read_data: Expr::x(),
            op_table: OpTableEntryVarArr::new_expr(),
            op_table_start_ptr: 0.into(),
            op_table_finish_ptr: 0.into(),
        }
        .into()
    }
}

/// Command type used in queue manager.
#[derive(Debug, Clone, Signal)]
pub struct Cmd<const QUEUE_INDEX_WIDTH: usize, const REQ_TAG_WIDTH: usize> {
    queue_ram_addr: Bits<U<QUEUE_INDEX_WIDTH>>,
    axil_reg: Bits<U<3>>,
    write_req: WReq,
    req_tag: Bits<U<REQ_TAG_WIDTH>>,
}

impl<const QUEUE_INDEX_WIDTH: usize, const REQ_TAG_WIDTH: usize> Cmd<QUEUE_INDEX_WIDTH, REQ_TAG_WIDTH> {
    /// Creates new expr.
    pub fn new_expr<'id>(
        queue_ram_addr: Expr<'id, Bits<U<QUEUE_INDEX_WIDTH>>>, axil_reg: Expr<'id, Bits<U<3>>>,
        write_req: Expr<'id, WReq>, req_tag: Expr<'id, Bits<U<REQ_TAG_WIDTH>>>,
    ) -> Expr<'id, Self> {
        CmdProj { queue_ram_addr, axil_reg, write_req, req_tag }.into()
    }
}

impl<const QUEUE_INDEX_WIDTH: usize, const REQ_TAG_WIDTH: usize> Command for Cmd<QUEUE_INDEX_WIDTH, REQ_TAG_WIDTH> {
    fn collision<'id>(lhs: Expr<'id, Self>, rhs: Expr<'id, Self>) -> Expr<'id, bool> {
        lhs.queue_ram_addr.is_eq(rhs.queue_ram_addr)
    }
}

#[derive(Debug, Clone, Signal)]
pub struct PipelineStage<Sel: Signal + Default, Cmd: Command> {
    selector: Sel,
    command: Cmd,
}

impl<const QUEUE_INDEX_WIDTH: usize, const REQ_TAG_WIDTH: usize>
    PipelineStage<Selector, Cmd<QUEUE_INDEX_WIDTH, REQ_TAG_WIDTH>>
{
    pub fn new_expr() -> Expr<'static, Self> {
        PipelineStageProj {
            selector: Selector::default().into(),
            command: Cmd::new_expr(0.into(), Expr::x(), WReq::new_expr(), Expr::x()),
        }
        .into()
    }
}

#[derive(Debug, Interface)]
pub struct ManagerInput<
    const AXIL_ADDR_WIDTH: usize,
    const QUEUE_INDEX_WIDTH: usize,
    const REQ_TAG_WIDTH: usize,
    const OP_TAG_WIDTH: usize,
> {
    /// Request input
    pub request: VrChannel<Req<QUEUE_INDEX_WIDTH, REQ_TAG_WIDTH>>,

    /// Commit input
    pub commit: VrChannel<CommitReq<OP_TAG_WIDTH>>,

    /// AXI-Lite slave interface
    #[member(nosep)]
    pub s_axil_aw: VrChannel<Addr<AXIL_ADDR_WIDTH>>,

    #[member(nosep)]
    pub s_axil_w: VrChannel<WReq>,

    #[member(nosep)]
    pub s_axil_ar: VrChannel<Addr<AXIL_ADDR_WIDTH>>,

    /// Configuration
    pub enable: UniChannel<bool>,
}

#[derive(Debug, Interface)]
pub struct ManagerOutput<Resp: Signal, Out: Signal> {
    /// Response
    pub response: VrChannel<Resp>,

    /// Output
    #[member(name = "response_out")]
    pub output: UniChannel<Valid<Out>>,

    /// AXI-Lite slave interface
    #[member(nosep)]
    pub s_axil_b: VrChannel<WRes>,

    #[member(nosep)]
    pub s_axil_r: VrChannel<RRes>,
}

#[derive(Debug, Clone, Signal)]
pub struct Temp<const QUEUE_INDEX_WIDTH: usize, const QUEUE_PTR_WIDTH: usize, Resp: Signal, Out: Signal> {
    op_table_start_entry: OpTableEntry<QUEUE_INDEX_WIDTH, QUEUE_PTR_WIDTH>,
    op_table_finish_entry: OpTableEntry<QUEUE_INDEX_WIDTH, QUEUE_PTR_WIDTH>,
    response: Valid<Resp>,
    output: Valid<Out>,
    s_axil_b: Valid<WRes>,
    s_axil_r: Valid<RRes>,
}
