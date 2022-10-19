use shakeflow::*;
use shakeflow_std::*;

pub type C<V: Signal> = VrChannel<V>;

pub fn m<V: Signal, const CREDIT_MAX: usize, const DECIMATION: usize>() -> Module<C<V>, C<V>> {
    composite::<C<V>, C<V>, _>("bsg_ready_to_credit_flow_converter", Some("i"), Some("o"), |input, k| {
        input.into_credit_flow::<CREDIT_MAX, DECIMATION>(k)
    })
    .build()
}
