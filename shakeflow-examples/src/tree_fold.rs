use shakeflow::*;
use shakeflow_std::*;

#[allow(clippy::type_complexity)]
pub fn m() -> Module<UniChannel<Array<Bits<U<8>>, U<64>>>, UniChannel<Bits<U<8>>>> {
    composite::<UniChannel<Array<Bits<U<8>>, U<64>>>, UniChannel<Bits<U<8>>>, _>(
        "tree_fold",
        Some("input"),
        Some("output"),
        |input, k| {
            input.map(k, |value| {
                let v_complex_init = !value[0] ^ value[1] & value[2];
                value.map(|v| !v).tree_fold(|l, r| l ^ r) | v_complex_init
            })
        },
    )
    .build()
}
