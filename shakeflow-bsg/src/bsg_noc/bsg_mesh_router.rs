use shakeflow::*;
use shakeflow_std::*;

use super::bsg_mesh_router_decoder_dor as mesh_router_decoder_dor;
// use super::pkg::{bsg_mesh_router as ruche, bsg_noc as noc};

#[derive(Debug, Interface)]
pub struct IC<const WIDTH: usize, const DIRS: usize, const X_CORD_WIDTH: usize, const Y_CORD_WIDTH: usize> {
    pub data: [VrChannel<Bits<U<WIDTH>>>; DIRS],
    pub my_x: UniChannel<Bits<U<X_CORD_WIDTH>>>,
    pub my_y: UniChannel<Bits<U<Y_CORD_WIDTH>>>,
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
>() -> Module<IC<WIDTH, DIRS, X_CORD_WIDTH, Y_CORD_WIDTH>, EC<WIDTH, DIRS>> {
    composite::<IC<WIDTH, DIRS, X_CORD_WIDTH, Y_CORD_WIDTH>, EC<WIDTH, DIRS>, _>(
        "bsg_mesh_router",
        Some("i"),
        Some("o"),
        |input, k| {
            let IC { data, my_x, my_y } = input;

            data.array_map_enumerate(|i, data| {
                let mut from = [false; 9];
                from[i] = true;

                let (data, data_fwd) = data.clone_uni(k);

                let dor_decoder = data_fwd
                    .zip3(k, my_x.clone(), my_y.clone())
                    .map(k, |input| {
                        let (data, my_x, my_y) = *input;

                        mesh_router_decoder_dor::IProj {
                            x_dirs: data.inner.clip_const::<U<X_CORD_WIDTH>>(0),
                            y_dirs: data.inner.clip_const::<U<Y_CORD_WIDTH>>(X_CORD_WIDTH),
                            my_x,
                            my_y,
                        }
                        .into()
                    })
                    .comb_inline(
                        k,
                        mesh_router_decoder_dor::m::<
                            X_CORD_WIDTH,
                            Y_CORD_WIDTH,
                            DIMS,
                            RUCHE_FACTOR_X,
                            RUCHE_FACTOR_Y,
                            XY_ORDER,
                            DEPOPULATED,
                        >(from),
                    )
                    .map(k, |input| input.resize::<U<DIRS>>())
                    .slice(k);

                data.duplicate_any(k)
                    .array_zip(dor_decoder)
                    .array_map(k, "req", |(data, temp_req), k| {
                        data.zip_uni(k, temp_req)
                            .and_then(k, None, |input| Expr::<Valid<_>>::new(input.1, input.0))
                            .buffer(k)
                    })
                    .arb_mux(k, 1, 0)
                    .map(k, |input| input.inner)
            })
        },
    )
    .build()
}
