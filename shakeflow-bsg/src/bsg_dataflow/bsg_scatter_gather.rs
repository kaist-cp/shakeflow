//! Given a bit vector, generates permutation vectors that perform concentration (fwd) and deconcentration (bkwd).
//!
//! ```text
//!           bit   fwd          bkwd
//!   pos.   vec    vec          vec
//!     3     1 --\    1      --- 2
//!                \         /
//!     2     1 -\  -> 3  <--  -- 1
//!               \           /
//!     1     0    --> 2  <---    3 --> 1
//!
//!     0     1 -----> 0  <------ 0
//! ```
//!
//! For empty slots; we just pick an unused slot, possible reusing the same empty slot multiple times. This allows
//! control logic to be unselected.

use shakeflow::*;
use shakeflow_std::*;

#[derive(Debug, Clone, Signal)]
pub struct E<const VEC_SIZE: usize> {
    fwd: Array<Bits<Log2<U<VEC_SIZE>>>, U<VEC_SIZE>>,
    fwd_datapath: Array<Bits<Log2<U<VEC_SIZE>>>, U<VEC_SIZE>>,
    bk: Array<Bits<Log2<U<VEC_SIZE>>>, U<VEC_SIZE>>,
    bk_datapath: Array<Bits<Log2<U<VEC_SIZE>>>, U<VEC_SIZE>>,
}

impl<const VEC_SIZE: usize> E<VEC_SIZE> {
    pub fn new_expr() -> Expr<'static, Self> {
        EProj {
            fwd: Expr::<Bits<Log2<U<VEC_SIZE>>>>::from(0).repeat(),
            fwd_datapath: Expr::<Bits<Log2<U<VEC_SIZE>>>>::from(0).repeat(),
            bk: Expr::<Bits<Log2<U<VEC_SIZE>>>>::from(0).repeat(),
            bk_datapath: Expr::<Bits<Log2<U<VEC_SIZE>>>>::from(0).repeat(),
        }
        .into()
    }
}

pub type IC<const VEC_SIZE: usize> = UniChannel<Bits<U<VEC_SIZE>>>;
pub type EC<const VEC_SIZE: usize> = UniChannel<E<VEC_SIZE>>;

/// Converts from an integer to a list of bit integers.
///
/// For example, `int_to_bit_list(1, 3)` is `[0, 0, 1]` and `int_to_bit_list(6, 3)` is `[1, 1, 0]`.
#[inline]
fn int_to_bit_list(a: usize, pad: usize) -> Vec<bool> {
    let mut b = format!("{a:b}").chars().map(|ch| ch == '1').collect::<Vec<bool>>();
    let mut v = vec![false; pad - b.len()];
    v.append(&mut b);
    v
}

/// Corresponds to `print_case_line`.
#[inline]
fn result_to_case_rhs<const VEC_SIZE: usize>(
    result: Vec<usize>,
) -> Expr<'static, Array<Bits<Log2<U<VEC_SIZE>>>, U<VEC_SIZE>>> {
    let case_rhs: [Expr<'static, Bits<Log2<U<VEC_SIZE>>>>; VEC_SIZE] = result
        .into_iter()
        .map(|a| {
            let mut rhs = int_to_bit_list(a, clog2(VEC_SIZE));
            rhs.resize(VEC_SIZE, false);
            Expr::<Bits<U<VEC_SIZE>>>::from(<[bool; VEC_SIZE] as TryFrom<_>>::try_from(rhs).unwrap()).resize()
        })
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    case_rhs.into()
}

/// Corresponds to `gen_{fwd|back}_vec_line_helper`.
#[inline]
fn gen_case_expr<const VEC_SIZE: usize>(a: usize) -> Expr<'static, E<VEC_SIZE>> {
    let l = int_to_bit_list(a, VEC_SIZE);

    let mut fwd = vec![];
    let mut fwd_datapath = vec![];
    let mut bk = vec![];
    let mut bk_datapath = vec![];
    // Initial value different from original code but doesn't matter;
    // if `spare` is never updated, it is never used
    let mut spare = 0;
    let mut pos = 0;
    let mut i_bk = 0;

    for (i, x) in l.into_iter().rev().enumerate() {
        if x {
            fwd.push(i);
            fwd_datapath.push(i - pos);
            pos += 1;
            bk.push(i_bk);
            bk_datapath.push(i_bk);
            i_bk += 1;
        } else {
            spare = i;
            bk.push(VEC_SIZE - 1);
            bk_datapath.push(0);
        }
    }

    for _ in 0..(VEC_SIZE - fwd.len()) {
        fwd.push(spare);
        fwd_datapath.push(0);
    }

    EProj {
        fwd: result_to_case_rhs(fwd),
        fwd_datapath: result_to_case_rhs(fwd_datapath),
        bk: result_to_case_rhs(bk),
        bk_datapath: result_to_case_rhs(bk_datapath),
    }
    .into()
}

pub fn m<const VEC_SIZE: usize>() -> Module<IC<VEC_SIZE>, EC<VEC_SIZE>> {
    composite::<IC<VEC_SIZE>, EC<VEC_SIZE>, _>("bsg_scatter_gather", Some("i"), Some("o"), |input, k| {
        input.fsm_map::<(), _, _>(k, None, ().into(), |input, state| {
            let case_items = (0..(1 << VEC_SIZE))
                .map(|i| (Expr::<Bits<U<VEC_SIZE>>>::from(i), gen_case_expr(i)))
                .collect::<Vec<_>>();

            let output = input.case(case_items, Some(Expr::x()));

            (output, state)
        })
    })
    .build()
}
