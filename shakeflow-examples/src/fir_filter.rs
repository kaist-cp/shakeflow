use shakeflow::*;
use shakeflow_std::*;

pub type FirFilterChannel<const SZ: usize> = UniChannel<Bits<U<SZ>>>;

pub fn fir_filter<const SZ: usize, const N: usize>(
    coeffs: [[bool; SZ]; N],
) -> Module<FirFilterChannel<SZ>, FirFilterChannel<SZ>> {
    composite::<FirFilterChannel<SZ>, FirFilterChannel<SZ>, _>("fir_filter", Some("in"), Some("out"), |value, k| {
        value
            .window::<N>(0.into(), k)
            .map(k, move |value| {
                value.zip(Array::new(coeffs.into_iter().map(Bits::<U<SZ>>::from).collect()).into()).map(|p| {
                    let (a, b) = *p;
                    (a * b).resize()
                })
            })
            .buffer(k, Expr::<Bits<U<SZ>>>::from(0).repeat())
            .map(k, move |value| value.sum())
    })
    .build()
}
