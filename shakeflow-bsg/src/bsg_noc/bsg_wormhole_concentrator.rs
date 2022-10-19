use shakeflow::*;
use shakeflow_std::*;

#[derive(Debug, Interface)]
pub struct C<const FLIT_WIDTH: usize, const N: usize> {
    // unconcentrated multiple links
    links: [VrChannel<Bits<U<FLIT_WIDTH>>>; N],
    // concentrated single link
    concentrated_link: VrChannel<Bits<U<FLIT_WIDTH>>>,
}

pub fn m<
    const FLIT_WIDTH: usize,
    const LEN_WIDTH: usize,
    const CID_WIDTH: usize,
    const CORD_WIDTH: usize,
    const N: usize,
>() -> Module<C<FLIT_WIDTH, N>, C<FLIT_WIDTH, N>> {
    composite::<C<FLIT_WIDTH, N>, C<FLIT_WIDTH, N>, _>("bsg_wormhole_concentrator", Some("i"), Some("o"), |input, k| {
        C {
            links: input.concentrated_link.comb_inline(
                k,
                super::bsg_wormhole_concentrator_out::m::<FLIT_WIDTH, LEN_WIDTH, CID_WIDTH, CORD_WIDTH, N>(),
            ),
            concentrated_link: input.links.comb_inline(
                k,
                super::bsg_wormhole_concentrator_in::m::<FLIT_WIDTH, LEN_WIDTH, CID_WIDTH, CORD_WIDTH, N>(),
            ),
        }
    })
    .build()
}
