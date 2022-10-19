//! Skid buffer for valid-ready channels.

use shakeflow_macro::Signal;

use crate::*;

/// Skid buffer's state.
#[derive(Debug, Clone, Signal)]
struct State<V: Signal> {
    /// Directly connected to module output.
    m_axis_data: Valid<V>,
    /// Temp register of skid buffer.
    temp_m_axis_data: Valid<V>,
    /// Datapath control.
    m_axis_ready_int: Ready,
}

/// Skid buffer's logic.
#[allow(clippy::type_complexity)]
fn logic<'id, V: Signal>(
    i_fwd: Expr<'id, Valid<V>>, o_bwd: Expr<'id, Ready>, state: Expr<'id, State<V>>,
) -> (Expr<'id, Valid<V>>, Expr<'id, Ready>, Expr<'id, State<V>>) {
    // Projections.
    let ValidProj { inner: skid_buffer_data_int, valid: skid_buffer_valid_int } = *i_fwd;

    let ValidProj { inner: m_axis_data_reg, valid: m_axis_valid_reg } = *state.m_axis_data;
    let ValidProj { inner: temp_m_axis_data_reg, valid: temp_m_axis_valid_reg } = *state.temp_m_axis_data;
    let ReadyProj { ready: m_axis_ready_int_reg } = *state.m_axis_ready_int;

    let m_axis_ready = o_bwd.ready;

    // Computes control path predicates.
    let m_axis_valid_int = skid_buffer_valid_int & m_axis_ready_int_reg;
    let m_axis_ready_int_early = m_axis_ready | (!temp_m_axis_valid_reg & (!m_axis_valid_reg | !m_axis_valid_int));

    let store_axis_int_to_output = m_axis_ready_int_reg & m_axis_ready | !m_axis_valid_reg;
    let store_axis_int_to_temp = m_axis_ready_int_reg & !m_axis_ready & m_axis_valid_reg;
    let store_axis_temp_to_output = !m_axis_ready_int_reg & m_axis_ready;

    // Computes next cycle state.
    let m_axis_data_next = select! {
        store_axis_int_to_output => skid_buffer_data_int,
        store_axis_temp_to_output => temp_m_axis_data_reg,
        default => m_axis_data_reg,
    };
    let temp_m_axis_data_next = store_axis_int_to_temp.cond(skid_buffer_data_int, temp_m_axis_data_reg);

    let m_axis_valid_next = m_axis_ready_int_reg.cond(
        (m_axis_ready | !m_axis_valid_reg).cond(m_axis_valid_int, m_axis_valid_reg),
        m_axis_ready.cond(temp_m_axis_valid_reg, m_axis_valid_reg),
    );
    let temp_m_axis_valid_next = m_axis_ready_int_reg.cond(
        (m_axis_ready | !m_axis_valid_reg).cond(temp_m_axis_valid_reg, m_axis_valid_int),
        !m_axis_ready & temp_m_axis_valid_reg,
    );

    // Mux for incoming packet
    let state_next = StateProj {
        m_axis_data: Expr::<Valid<_>>::new(m_axis_valid_next, m_axis_data_next),
        temp_m_axis_data: Expr::<Valid<_>>::new(temp_m_axis_valid_next, temp_m_axis_data_next),
        m_axis_ready_int: Expr::<Ready>::new(m_axis_ready_int_early),
    }
    .into();

    (state.m_axis_data, state.m_axis_ready_int, state_next)
}

/// Creates a skid buffer module.
fn m<V: Signal, const P: Protocol>() -> Module<VrChannel<V, P>, VrChannel<V>> {
    composite::<VrChannel<V, P>, VrChannel<V>, _>("buffer_skid", Some("in"), Some("out"), |value, k| {
        value.fsm(
            k,
            None,
            StateProj {
                m_axis_data: Expr::invalid(),
                temp_m_axis_data: Expr::invalid(),
                m_axis_ready_int: ReadyProj { ready: false.into() }.into(),
            }
            .into(),
            logic,
        )
    })
    .build()
}

impl<I: Signal, const P: Protocol> VrChannel<I, P> {
    /// Adds a skid buffer.
    pub fn buffer_skid(self, k: &mut CompositeModuleContext) -> VrChannel<I> { self.comb_inline(k, m()) }
}
