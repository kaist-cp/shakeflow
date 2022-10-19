//! Round-robin mux.

use shakeflow::*;

use crate::*;

/// Round-robin mux that muxes an interface out of N input interfaces.
pub trait RrMuxExt<I: Interface, const N: usize> {
    /// TODO: Documentation
    fn rr_mux(self, k: &mut CompositeModuleContext) -> (UniChannel<Bits<Log2<U<N>>>>, I);
}

fn m_rr_mux_vr<V: Signal, const N: usize>() -> Module<[VrChannel<V>; N], (UniChannel<Bits<Log2<U<N>>>>, VrChannel<V>)> {
    composite::<(UniChannel<Bits<Log2<U<N>>>>, [VrChannel<V>; N]), (UniChannel<Bits<Log2<U<N>>>>, VrChannel<V>), _>(
        "rr_mux",
        Some("in"),
        Some("out"),
        |(tag, input), k| {
            let output = (tag, input).mux(k);
            let (output, fire) = output.fire(k);
            let tag = fire.counter::<U<N>>(k).map(k, |input| input.0);
            (tag, output)
        },
    )
    .wrap(|_, input, (tag, output)| ((tag.clone(), input), (tag, output)))
    .build()
}

impl<V: Signal, const N: usize> RrMuxExt<VrChannel<V>, N> for [VrChannel<V>; N] {
    fn rr_mux(self, k: &mut CompositeModuleContext) -> (UniChannel<Bits<Log2<U<N>>>>, VrChannel<V>) {
        self.comb_inline(k, m_rr_mux_vr::<V, N>())
    }
}
