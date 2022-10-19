//! TODO: This implementation is wrong. We need to implement `clip` API with usize length.

use shakeflow::*;
use shakeflow_std::*;

const DIMS: usize = 2;
const OUTPUT_DIRS: usize = DIMS * 2 + 1;
const CORD_MARKERS_POS: [usize; 3] = [0, 4, 5];

#[derive(Debug, Clone, Signal)]
pub struct I {
    target_cord: Bits<U<OUTPUT_DIRS>>,
    my_cord: Bits<U<OUTPUT_DIRS>>,
}

pub type IC = UniChannel<I>;
pub type EC = UniChannel<Bits<U<OUTPUT_DIRS>>>;

pub fn m<const REVERSE_ORDER: bool>() -> Module<IC, EC> {
    composite::<IC, EC, _>("bsg_wormhole_router_decoder_dor", Some("i"), Some("req_o"), |input, k| {
        input.map(k, |input| {
            let target_cord_i = input.target_cord;
            let my_cord_i = input.my_cord;

            let (eq, (lt, gt)): (Vec<_>, (Vec<_>, Vec<_>)) = CORD_MARKERS_POS
                .windows(2)
                .map(|window| {
                    let (lower, upper) = (window[0], window[1]);
                    let len = upper - lower;

                    // TODO: Implement `clip` API with non-constant usize length, with constant maximum length.
                    let targ_cord = target_cord_i % (Expr::<Bits<U<5>>>::from(1) << len);
                    let my_cord = my_cord_i % (Expr::from(1) << len);

                    let eq = targ_cord.is_eq(my_cord);
                    let lt = targ_cord.is_lt(my_cord);
                    let gt = !eq & !lt;

                    (eq, (lt, gt))
                })
                .unzip();

            let eq: Expr<Bits<U<DIMS>>> = <[Expr<bool>; DIMS] as TryFrom<_>>::try_from(eq).unwrap().into();
            let lt: Expr<Bits<U<DIMS>>> = <[Expr<bool>; DIMS] as TryFrom<_>>::try_from(lt).unwrap().into();
            let gt: Expr<Bits<U<DIMS>>> = <[Expr<bool>; DIMS] as TryFrom<_>>::try_from(gt).unwrap().into();

            // handle base case
            let req: [Expr<bool>; OUTPUT_DIRS] = if REVERSE_ORDER {
                [
                    vec![eq.all()],
                    (0..(DIMS - 1))
                        .flat_map(|i| {
                            let mask_upper = Expr::<Bits<U<DIMS>>>::from((1 << (i + 1)) - 1);
                            let mask_lower = Expr::<Bits<U<DIMS>>>::from((1 << i) - 1);
                            vec![
                                (eq & mask_upper).is_eq(mask_lower) & lt[i],
                                (eq & mask_upper).is_eq(mask_lower) & gt[i],
                            ]
                        })
                        .collect(),
                    vec![lt[DIMS - 1], gt[DIMS - 1]],
                ]
                .concat()
            } else {
                [
                    vec![eq.all()],
                    vec![lt[0], gt[0]],
                    (1..DIMS)
                        .flat_map(|i| {
                            let mask_upper = Expr::<Bits<U<DIMS>>>::from((1 << (i + 1)) - 1);
                            let mask_lower = Expr::<Bits<U<DIMS>>>::from((1 << i) - 1);
                            vec![
                                (eq & mask_upper).is_eq(mask_lower) & lt[i],
                                (eq & mask_upper).is_eq(mask_lower) & gt[i],
                            ]
                        })
                        .collect(),
                ]
                .concat()
            }
            .try_into()
            .unwrap();

            req.into()
        })
    })
    .build()
}
