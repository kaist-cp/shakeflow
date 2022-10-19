//! bsg_mesh_to_ring_stitch
//!
//! This module uses space filling curves to organize a mesh of items into a ring of nearest-neighbor
//! connections. Because of geometry, both X and Y coordinates cannot be odd.

use std::collections::HashMap;

use shakeflow::*;
use shakeflow_std::*;

const Y_MAX: usize = 2;
const X_MAX: usize = 2;
const B: usize = clog2(X_MAX * Y_MAX);

#[derive(Debug, Clone, Signal)]
pub struct I<const WIDTH_BACK: usize, const WIDTH_FWD: usize> {
    back_data_out: Array<Array<Bits<U<WIDTH_BACK>>, U<Y_MAX>>, U<X_MAX>>,
    fwd_data_out: Array<Array<Bits<U<WIDTH_FWD>>, U<Y_MAX>>, U<X_MAX>>,
}

#[derive(Debug, Clone, Signal)]
pub struct E<const WIDTH_BACK: usize, const WIDTH_FWD: usize> {
    id: Array<Array<Bits<U<B>>, U<Y_MAX>>, U<X_MAX>>,
    back_data_in: Array<Array<Bits<U<WIDTH_BACK>>, U<Y_MAX>>, U<X_MAX>>,
    fwd_data_in: Array<Array<Bits<U<WIDTH_FWD>>, U<Y_MAX>>, U<X_MAX>>,
}

pub type IC<const WIDTH_BACK: usize, const WIDTH_FWD: usize> = UniChannel<I<WIDTH_BACK, WIDTH_FWD>>;
pub type EC<const WIDTH_BACK: usize, const WIDTH_FWD: usize> = UniChannel<E<WIDTH_BACK, WIDTH_FWD>>;

fn get_order(max_x: usize, max_y: usize) -> Vec<(usize, usize)> {
    match (max_x, max_y) {
        (1, 2) => vec![(0, 0), (0, 1)],
        (2, 1) => vec![(0, 0), (1, 0)],
        (max_x, max_y) if max_x % 2 == 0 => ::std::iter::empty()
            .chain((0..max_x).step_by(2).flat_map(|x| {
                ::std::iter::empty()
                    .chain((1..max_y).map(move |y| (x, y)))
                    .chain((1..max_y).rev().map(move |y| (x + 1, y)))
            }))
            .chain((0..max_x).rev().map(|x| (x, 0)))
            .collect::<Vec<_>>(),
        (max_x, max_y) if max_y % 2 == 0 => ::std::iter::empty()
            .chain((0..max_y).step_by(2).flat_map(|y| {
                ::std::iter::empty()
                    .chain((1..max_x).map(move |x| (x, y)))
                    .chain((1..max_x).rev().map(move |x| (x, y + 1)))
            }))
            .chain((0..max_y).rev().map(|y| (0, y)))
            .collect::<Vec<_>>(),
        _ => panic!("invalid max_x/max_y"),
    }
}

pub fn m<const WIDTH_BACK: usize, const WIDTH_FWD: usize>(
) -> Module<IC<WIDTH_BACK, WIDTH_FWD>, EC<WIDTH_BACK, WIDTH_FWD>> {
    composite::<IC<WIDTH_BACK, WIDTH_FWD>, EC<WIDTH_BACK, WIDTH_FWD>, _>(
        "bsg_mesh_to_ring_stitch",
        Some("i"),
        Some("o"),
        |input, k| {
            let order = get_order(X_MAX, Y_MAX);

            let mut matrix = vec![vec![0; Y_MAX]; X_MAX];
            let mut my_dict = HashMap::new();

            for (pos, (x, y)) in order.into_iter().enumerate() {
                my_dict.insert(pos, (x, y));
                matrix[x][y] = pos;
            }

            input.map(k, move |input| {
                let mut back_data_in: Expr<Array<Array<Bits<U<WIDTH_BACK>>, U<Y_MAX>>, U<X_MAX>>> = Expr::x();
                let mut fwd_data_in: Expr<Array<Array<Bits<U<WIDTH_FWD>>, U<Y_MAX>>, U<X_MAX>>> = Expr::x();
                let mut id: Expr<Array<Array<Bits<U<B>>, U<Y_MAX>>, U<X_MAX>>> = Expr::x();

                let back_data_out = input.back_data_out;
                let fwd_data_out = input.fwd_data_out;

                for y in (0..Y_MAX).rev() {
                    for x in (0..X_MAX).rev() {
                        let pos = matrix[x][y];

                        let below = if pos < 1 { X_MAX * Y_MAX - 1 } else { pos - 1 };

                        let above = if pos + 1 == X_MAX * Y_MAX { 0 } else { pos + 1 };

                        let (below_x, below_y) = my_dict.get(&below).unwrap();
                        let (above_x, above_y) = my_dict.get(&above).unwrap();

                        back_data_in = back_data_in
                            .set((*below_x).into(), back_data_in[*below_x].set((*below_y).into(), back_data_out[x][y]));
                        fwd_data_in = fwd_data_in
                            .set((*above_x).into(), fwd_data_in[*above_x].set((*above_y).into(), fwd_data_out[x][y]));
                    }
                }

                for x in 0..X_MAX {
                    for y in 0..Y_MAX {
                        id = id.set(x.into(), id[x].set(y.into(), matrix[x][y].into()));
                    }
                }

                EProj { id, back_data_in, fwd_data_in }.into()
            })
        },
    )
    .build()
}
