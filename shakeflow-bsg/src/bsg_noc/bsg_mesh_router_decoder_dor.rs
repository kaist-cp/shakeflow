use shakeflow::*;
use shakeflow_std::*;

use super::pkg::{bsg_mesh_router as ruche, bsg_noc as noc};

#[derive(Debug, Clone, Signal)]
pub struct I<const X_CORD_WIDTH: usize, const Y_CORD_WIDTH: usize> {
    x_dirs: Bits<U<X_CORD_WIDTH>>,
    y_dirs: Bits<U<Y_CORD_WIDTH>>,

    my_x: Bits<U<X_CORD_WIDTH>>,
    my_y: Bits<U<Y_CORD_WIDTH>>,
}

pub type Dirs<const DIMS: usize> = Sum<Prod<U<DIMS>, U<2>>, U<1>>;

pub type IC<const X_CORD_WIDTH: usize, const Y_CORD_WIDTH: usize> = UniChannel<I<X_CORD_WIDTH, Y_CORD_WIDTH>>;
pub type EC<const DIMS: usize> = UniChannel<Bits<Dirs<DIMS>>>;

pub fn m<
    const X_CORD_WIDTH: usize,
    const Y_CORD_WIDTH: usize,
    const DIMS: usize,
    const RUCHE_FACTOR_X: usize,
    const RUCHE_FACTOR_Y: usize,
    const XY_ORDER: bool,
    const DEPOPULATED: bool,
>(
    from: [bool; 9],
) -> Module<IC<X_CORD_WIDTH, Y_CORD_WIDTH>, EC<DIMS>> {
    composite::<IC<X_CORD_WIDTH, Y_CORD_WIDTH>, EC<DIMS>, _>(
        "bsg_mesh_router_decoder_dor",
        Some("i"),
        Some("reqs_o"),
        move |input, k| {
            input.map(k, move |input| {
                let x_dirs = input.x_dirs;
                let y_dirs = input.y_dirs;
                let my_x = input.my_x;
                let my_y = input.my_y;

                // compare coordinates
                let x_eq = x_dirs.is_eq(my_x);
                let y_eq = y_dirs.is_eq(my_y);
                let x_gt = x_dirs.is_gt(my_x);
                let y_gt = y_dirs.is_gt(my_y);
                let x_lt = !x_gt & !x_eq;
                let y_lt = !y_gt & !y_eq;

                // P-port
                let req_p = x_eq & y_eq;

                let (req_w, req_rw, req_e, req_re) = if RUCHE_FACTOR_X > 0 {
                    if XY_ORDER {
                        let re_cord = my_x + RUCHE_FACTOR_X.into();

                        let (send_rw, send_re) = if DEPOPULATED {
                            (
                                my_x.is_gt(RUCHE_FACTOR_X.into()) & x_dirs.is_lt(my_x - RUCHE_FACTOR_X.into()),
                                !re_cord[X_CORD_WIDTH] & x_dirs.is_gt(re_cord.resize()),
                            )
                        } else {
                            (
                                my_x.is_ge(RUCHE_FACTOR_X.into()) & x_dirs.is_le(my_x - RUCHE_FACTOR_X.into()),
                                !re_cord[X_CORD_WIDTH] & x_dirs.is_ge(re_cord.resize()),
                            )
                        };

                        (x_lt & !send_rw, send_rw, x_gt & !send_re, send_re)
                    } else {
                        let dxp: Expr<Bits<U<X_CORD_WIDTH>>> = (x_dirs - my_x) % RUCHE_FACTOR_X.into();
                        let dxn: Expr<Bits<U<X_CORD_WIDTH>>> = (my_x - x_dirs) % RUCHE_FACTOR_X.into();

                        let req_w = y_eq & x_lt & dxn.is_gt(0.into());
                        let req_rw = y_eq & x_lt & dxn.is_eq(0.into());
                        let req_e = y_eq & x_gt & dxp.is_gt(0.into());
                        let req_re = y_eq & x_gt & dxp.is_eq(0.into());

                        if from[noc::Dirs::S as usize] | from[noc::Dirs::N as usize] | from[noc::Dirs::P as usize] {
                            if DEPOPULATED {
                                (y_eq & x_lt, false.into(), y_eq & x_gt, false.into())
                            } else {
                                (req_w, req_rw, req_e, req_re)
                            }
                        } else if from[noc::Dirs::W as usize] {
                            (false.into(), false.into(), req_e, req_re)
                        } else if from[noc::Dirs::E as usize] {
                            (req_w, req_rw, false.into(), false.into())
                        } else if from[ruche::RucheDirs::RW as usize] {
                            (false.into(), false.into(), false.into(), y_eq & x_gt)
                        } else if from[ruche::RucheDirs::RE as usize] {
                            (false.into(), y_eq & x_lt, false.into(), false.into())
                        } else if from[ruche::RucheDirs::RN as usize] | from[ruche::RucheDirs::RS as usize] {
                            if DEPOPULATED {
                                (false.into(), false.into(), false.into(), false.into())
                            } else {
                                (req_w, req_rw, req_e, req_re)
                            }
                        } else {
                            (false.into(), false.into(), false.into(), false.into())
                        }
                    }
                } else if XY_ORDER {
                    (x_lt, false.into(), x_gt, false.into())
                } else {
                    (y_eq & x_lt, false.into(), y_eq & x_gt, false.into())
                };

                let (req_n, req_rn, req_s, req_rs) = if RUCHE_FACTOR_Y > 0 {
                    if XY_ORDER {
                        let rs_cord = my_y + RUCHE_FACTOR_Y.into();

                        let (send_rn, send_rs) = if DEPOPULATED {
                            (
                                my_y.is_gt(RUCHE_FACTOR_Y.into()) & y_dirs.is_lt(my_y - RUCHE_FACTOR_Y.into()),
                                !rs_cord[Y_CORD_WIDTH] & y_dirs.is_gt(rs_cord.resize()),
                            )
                        } else {
                            (
                                my_y.is_ge(RUCHE_FACTOR_Y.into()) & y_dirs.is_le(my_y - RUCHE_FACTOR_Y.into()),
                                !rs_cord[Y_CORD_WIDTH] & y_dirs.is_ge(rs_cord.resize()),
                            )
                        };

                        (y_lt & !send_rn, send_rn, y_gt & !send_rs, send_rs)
                    } else {
                        let dyp: Expr<Bits<U<Y_CORD_WIDTH>>> = (y_dirs - my_y) % RUCHE_FACTOR_Y.into();
                        let r#dyn: Expr<Bits<U<Y_CORD_WIDTH>>> = (my_y - y_dirs) % RUCHE_FACTOR_Y.into();

                        let req_n = x_eq & y_lt & r#dyn.is_gt(0.into());
                        let req_rn = x_eq & y_lt & r#dyn.is_eq(0.into());
                        let req_s = x_eq & y_gt & dyp.is_gt(0.into());
                        let req_rs = x_eq & y_gt & dyp.is_eq(0.into());

                        if from[noc::Dirs::E as usize] | from[noc::Dirs::W as usize] | from[noc::Dirs::P as usize] {
                            (req_n, req_rn, req_s, req_rs)
                        } else if from[noc::Dirs::N as usize] {
                            (false.into(), false.into(), req_s, req_rs)
                        } else if from[noc::Dirs::S as usize] {
                            (req_n, req_rn, false.into(), false.into())
                        } else if from[ruche::RucheDirs::RN as usize] {
                            (false.into(), false.into(), false.into(), x_eq & y_gt)
                        } else if from[ruche::RucheDirs::RS as usize] {
                            (false.into(), x_eq & y_lt, false.into(), false.into())
                        } else if from[ruche::RucheDirs::RW as usize] | from[ruche::RucheDirs::RE as usize] {
                            if DEPOPULATED {
                                // If depopulated, there wouldn't be these paths.
                                (false.into(), false.into(), false.into(), false.into())
                            } else {
                                (req_n, req_rn, req_s, req_rs)
                            }
                        } else {
                            (false.into(), false.into(), false.into(), false.into())
                        }
                    }
                } else if XY_ORDER {
                    (y_lt, false.into(), y_gt, false.into())
                } else {
                    (x_eq & y_lt, false.into(), x_eq & y_gt, false.into())
                };

                Expr::<Bits<U<9>>>::from([req_p, req_w, req_n, req_e, req_s, req_rw, req_rn, req_re, req_rs]).resize()
            })
        },
    )
    .build()
}
