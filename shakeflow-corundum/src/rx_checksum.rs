use shakeflow::num::*;
use shakeflow::*;
use shakeflow_std::axis::*;
use shakeflow_std::*;

use super::types::checksum::*;

pub const DATA_WIDTH: usize = 512;
pub const KEEP_WIDTH: usize = DATA_WIDTH / 8;
pub const START_OFFSET: usize = 14;

pub type I<const DATA_WIDTH: usize, const KEEP_WIDTH: usize> = AxisChannel<Keep<U<DATA_WIDTH>, U<KEEP_WIDTH>>>;
pub type O = UniChannel<Valid<Bits<U<16>>>>;

// TODO: Make this parameterizable
const OFFSET_WIDTH: usize = 1;

#[derive(Debug, Clone, Signal)]
struct Mask<const KEEP_WIDTH: usize> {
    offset: Bits<U<OFFSET_WIDTH>>,
    mask: Bits<U<KEEP_WIDTH>>,
}

impl<const KEEP_WIDTH: usize> Mask<KEEP_WIDTH> {
    // TODO: Make this function `const fn`.
    fn new() -> Self {
        let offset: Bits<U<OFFSET_WIDTH>> = usize_to_bits(START_OFFSET / KEEP_WIDTH).into();
        let mask: Bits<U<KEEP_WIDTH>> = usize_to_bits((((1u128 << KEEP_WIDTH) - 1) << START_OFFSET) as usize).into();
        Self { offset, mask }
    }
}

fn csum_adder<N: Num, W: Num>(
    k: &mut CompositeModuleContext, input: UniChannel<Valid<AxisValue<Array<Bits<N>, W>>>>,
) -> UniChannel<Valid<AxisValue<Array<Bits<Sum<N, U<1>>>, Quot<W, U<2>>>>>> {
    input
        .map_inner(k, |value| value.map(|payload| payload.chunk::<U<2>>().map(|b| b[0] + b[1])))
        .buffer(k, Expr::invalid())
}

pub fn m(module_name: &str) -> Module<I<DATA_WIDTH, KEEP_WIDTH>, O> {
    composite::<I<DATA_WIDTH, KEEP_WIDTH>, _, _>(module_name, Some("s_axis"), Some("m_axis_csum"), |value, k| {
        let pipeline_input = value
            .into_vr(k)
            // Masks input.
            .fsm_map::<Mask<KEEP_WIDTH>, _, _>(k, None, Mask::new().into(), |input, state| {
                let payload = input.payload;

                // Masks data with keep and the state's mask.
                let payload = payload.tdata
                    & (payload.tkeep & state.mask).map(|b| b.repeat::<U<8>>()).concat().resize();
                let output = AxisValueProj { payload: payload.chunk::<U<16>>(), tlast: input.tlast };

                // Updates the state.
                let finished = Expr::from(OFFSET_WIDTH == 0) | state.offset.is_eq(0.into());
                let finishing = state.offset.is_eq(1.into());
                let state = input.tlast.cond(
                    Mask::new().into(),
                    MaskProj {
                        offset: finished.cond(state.offset, state.offset - 1.into()),
                        mask: (!finishing).cond(
                            finished.repeat::<U<KEEP_WIDTH>>(),
                            Expr::from(true).repeat::<U<KEEP_WIDTH>>() << (START_OFFSET % KEEP_WIDTH),
                        ),
                    }
                    .into(),
                );

                (output.into(), state)
            })
            // Transforms into unidirectional channel.
            .into_uni(k, true)
            .map_inner(k, |value| {
                value.map(|payload| payload.map(|b| {
                    Expr::<Bits<num::U<16>>>::from(b.clip_const::<U<8>>(8).append(b.clip_const::<U<8>>(0)))
                }))
            });

        let pipeline = csum_adder(k, pipeline_input);
        let pipeline = csum_adder(k, pipeline);
        let pipeline = csum_adder(k, pipeline);
        let pipeline = csum_adder(k, pipeline);
        let pipeline = csum_adder(k, pipeline);

        pipeline
            .map_inner(k, |value| {
                value.map(|payload| {
                    let data = payload.repr();
                    let data = data.clip_const::<U<16>>(0) + data.clip_const::<U<5>>(16).resize();
                    let data = data.clip_const::<U<16>>(0) + data.clip_const::<U<1>>(16).resize();
                    data.resize::<U<16>>()
                })
            })
            .fsm_map::<Accumulator, _, _>(k, None, Accumulator::default().into(), |input, state| {
                let value = input.inner;
                let valid = input.valid;

                let sum = state.sum + value.payload;
                let sum = sum.clip_const::<U<16>>(0) + sum.clip_const::<U<1>>(16).resize::<U<16>>();
                let sum = sum.resize::<U<16>>();

                let next_state = (!valid)
                    .cond(state, value.tlast.cond(Expr::from(Accumulator::default()), AccumulatorProj { sum }.into()));
                (Expr::<Valid<_>>::new(valid & value.tlast, sum), next_state)
            })
            .buffer(k, Expr::invalid())
    })
    .build()
}
