use shakeflow::*;
use shakeflow_std::*;

#[derive(Debug, Interface)]
pub struct C<V: Signal> {
    side_a_links: VrChannel<V>,
    side_b_links: VrChannel<V>,
}

pub fn m<V: Signal>() -> Module<C<V>, C<V>> {
    composite::<C<V>, C<V>, _>("bsg_noc_repeater_node", Some("i"), Some("o"), |input, k| {
        C {
            side_a_links: input.side_b_links.fifo::<2>(k), // B to A
            side_b_links: input.side_a_links.fifo::<2>(k), // A to B
        }
    })
    .build()
}
