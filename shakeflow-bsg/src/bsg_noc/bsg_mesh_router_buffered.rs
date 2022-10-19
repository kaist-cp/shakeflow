use shakeflow::*;
use shakeflow_std::*;

use super::bsg_mesh_router as mesh_router;

#[derive(Debug, Interface)]
pub struct IC<const WIDTH: usize, const DIRS: usize, const X_CORD_WIDTH: usize, const Y_CORD_WIDTH: usize> {
    link: [VrChannel<Bits<U<WIDTH>>>; DIRS],
    my_x: UniChannel<Bits<U<X_CORD_WIDTH>>>,
    my_y: UniChannel<Bits<U<Y_CORD_WIDTH>>>,
}

pub type EC<const WIDTH: usize, const DIRS: usize> = [VrChannel<Bits<U<WIDTH>>>; DIRS];

pub fn m<
    const WIDTH: usize,
    const X_CORD_WIDTH: usize,
    const Y_CORD_WIDTH: usize,
    const RUCHE_FACTOR_X: usize,
    const RUCHE_FACTOR_Y: usize,
    const DIMS: usize,
    const DIRS: usize,
    const XY_ORDER: bool,
    const DEPOPULATED: bool,
>(
    use_credits: [bool; DIRS],
) -> Module<IC<WIDTH, DIRS, X_CORD_WIDTH, Y_CORD_WIDTH>, EC<WIDTH, DIRS>> {
    composite::<IC<WIDTH, DIRS, X_CORD_WIDTH, Y_CORD_WIDTH>, EC<WIDTH, DIRS>, _>(
        "bsg_mesh_router_buffered",
        Some("i"),
        Some("o"),
        |input, k| {
            let IC { link, my_x, my_y } = input;

            let fifo = link.array_map_enumerate(|i, ch| {
                if use_credits[i] {
                    ch.comb_inline(
                        k,
                        crate::bsg_dataflow::bsg_fifo_1r1w_small_credit_on_input::m::<Bits<U<WIDTH>>, 2, false>(),
                    )
                } else {
                    ch.comb_inline(k, crate::bsg_dataflow::bsg_fifo_1r1w_small::m::<Bits<U<WIDTH>>, 2, false>())
                }
            });

            mesh_router::IC { data: fifo, my_x, my_y }.comb_inline(
                k,
                mesh_router::m::<
                    WIDTH,
                    X_CORD_WIDTH,
                    Y_CORD_WIDTH,
                    RUCHE_FACTOR_X,
                    RUCHE_FACTOR_Y,
                    DIMS,
                    DIRS,
                    XY_ORDER,
                    DEPOPULATED,
                >(),
            )
        },
    )
    .build()
}
