use shakeflow::*;
use shakeflow_std::*;

#[derive(Debug, Clone, Signal)]
pub struct E {
    data: Valid<Bits<U<8>>>,
    k: bool,
    frame_align: bool,
}

pub type IC = UniChannel<bool>;
pub type EC = UniChannel<E>;

#[derive(Debug, Clone, Signal)]
pub struct S {
    decode_rd: bool,
    shift_reg: Bits<U<10>>,
    frame_counter: Bits<U<4>>,
}

impl S {
    /// Creates new expr.
    pub fn new_expr() -> Expr<'static, S> {
        SProj { decode_rd: false.into(), shift_reg: 0.into(), frame_counter: 0.into() }.into()
    }
}

pub fn m() -> Module<IC, EC> {
    composite::<IC, EC, _>("bsg_8b10b_shift_decoder", Some("i"), Some("o"), |input, k| {
        input.fsm_map::<S, E, _>(k, None, S::new_expr(), |input, state| {
            // Input Shift Register
            let shift_reg_next = state.shift_reg.clip_const::<U<9>>(1).append(input.repr()).resize();

            // Comma code detection and Frame alignment
            let comma_code_rdn = state.shift_reg.clip_const::<U<7>>(0).is_eq(0b1111100.into());
            let comma_code_rdp = state.shift_reg.clip_const::<U<7>>(0).is_eq(0b0000011.into());
            let frame_align = comma_code_rdn | comma_code_rdp;

            // Frame counter
            let frame_recv = state.frame_counter.is_eq(9.into());
            let frame_counter_next =
                (frame_recv | frame_align).cond(0.into(), (state.frame_counter + 1.into()).resize());

            // 8b/10b decoder
            let decoder_output = super::bsg_8b10b_decode_comb::logic(
                super::bsg_8b10b_decode_comb::IProj { data: state.shift_reg, rd: state.decode_rd }.into(),
            );
            let valid = frame_recv & !(decoder_output.data_err | decoder_output.rd_err);

            let decode_rd_next = frame_align.cond(comma_code_rdn, valid.cond(decoder_output.rd, state.decode_rd));

            let output =
                EProj { data: Expr::<Valid<_>>::new(valid, decoder_output.data), k: decoder_output.k, frame_align }
                    .into();
            let state_next =
                SProj { decode_rd: decode_rd_next, shift_reg: shift_reg_next, frame_counter: frame_counter_next }
                    .into();

            (output, state_next)
        })
    })
    .build()
}
