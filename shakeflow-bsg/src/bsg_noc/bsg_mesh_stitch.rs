//! Stitches together wires according to a mesh topology. Edges are returned in `hor` and `ver` arrays.
//!
//! TODO: Use `pkg::bsg_noc` for the direction instead of magic number

use shakeflow::*;
use shakeflow_std::*;

#[derive(Debug, Interface)]
pub struct IC<const WIDTH: usize, const X_MAX: usize, const Y_MAX: usize, const NETS: usize> {
    outs: [[[[UniChannel<Bits<U<WIDTH>>>; 4]; NETS]; X_MAX]; Y_MAX],

    hor: [[[UniChannel<Bits<U<WIDTH>>>; NETS]; Y_MAX]; 2],
    ver: [[[UniChannel<Bits<U<WIDTH>>>; NETS]; X_MAX]; 2],
}

#[derive(Debug, Interface)]
pub struct EC<const WIDTH: usize, const X_MAX: usize, const Y_MAX: usize, const NETS: usize> {
    ins: [[[[UniChannel<Bits<U<WIDTH>>>; 4]; NETS]; X_MAX]; Y_MAX],

    hor: [[[UniChannel<Bits<U<WIDTH>>>; NETS]; Y_MAX]; 2],
    ver: [[[UniChannel<Bits<U<WIDTH>>>; NETS]; X_MAX]; 2],
}

pub fn m<const WIDTH: usize, const X_MAX: usize, const Y_MAX: usize, const NETS: usize>(
) -> Module<IC<WIDTH, X_MAX, Y_MAX, NETS>, EC<WIDTH, X_MAX, Y_MAX, NETS>> {
    composite::<IC<WIDTH, X_MAX, Y_MAX, NETS>, EC<WIDTH, X_MAX, Y_MAX, NETS>, _>(
        "bsg_mesh_stitch",
        Some("i"),
        Some("o"),
        |input, _| {
            let IC { outs: outs_i, hor: hor_i, ver: ver_i } = input;

            let ins_o = range_map::<Y_MAX, _, _>(|r| {
                range_map::<X_MAX, _, _>(|c| {
                    range_map::<NETS, _, _>(|net| {
                        [
                            if c == 0 { hor_i[0][r][net].clone() } else { outs_i[r][0][net][1].clone() },
                            if c == X_MAX - 1 {
                                hor_i[1][r][net].clone()
                            } else {
                                outs_i[r][X_MAX - 1][net][0].clone()
                            },
                            if r == 0 { ver_i[0][c][net].clone() } else { outs_i[r][c][net][3].clone() },
                            if r == Y_MAX - 1 { ver_i[1][c][net].clone() } else { outs_i[r][c][net][2].clone() },
                        ]
                    })
                })
            });

            let hor_o = [
                range_map::<Y_MAX, _, _>(|r| range_map::<NETS, _, _>(|net| outs_i[r][0][net][0].clone())),
                range_map::<Y_MAX, _, _>(|r| range_map::<NETS, _, _>(|net| outs_i[r][X_MAX - 1][net][1].clone())),
            ];

            let ver_o = [
                range_map::<X_MAX, _, _>(|c| range_map::<NETS, _, _>(|net| outs_i[0][c][net][2].clone())),
                range_map::<X_MAX, _, _>(|c| range_map::<NETS, _, _>(|net| outs_i[Y_MAX - 1][c][net][3].clone())),
            ];

            EC { ins: ins_o, hor: hor_o, ver: ver_o }
        },
    )
    .build()
}
