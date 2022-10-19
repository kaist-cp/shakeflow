use shakeflow::*;
use shakeflow_std::axis::*;

#[allow(clippy::type_complexity)]
pub fn m<const INPUT_WIDTH: usize, const SPLIT_WIDTH: usize>(
    name: &str,
) -> Module<
    AxisChannel<Keep<U<INPUT_WIDTH>, Quot<U<INPUT_WIDTH>, U<8>>>>,
    (
        AxisVrChannel<Keep<U<SPLIT_WIDTH>, Quot<U<SPLIT_WIDTH>, U<8>>>>,
        AxisChannel<Keep<U<INPUT_WIDTH>, Quot<U<INPUT_WIDTH>, U<8>>>>,
    ),
> {
    composite::<
        AxisChannel<Keep<U<INPUT_WIDTH>, Quot<U<INPUT_WIDTH>, U<8>>>>,
        (
            AxisVrChannel<Keep<U<SPLIT_WIDTH>, Quot<U<SPLIT_WIDTH>, U<8>>>>,
            AxisChannel<Keep<U<INPUT_WIDTH>, Quot<U<INPUT_WIDTH>, U<8>>>>,
        ),
        _,
    >(name, Some("i"), Some("o"), |i, k| {
        let (o1, o2) = i.split::<U<SPLIT_WIDTH>>(k);
        (o1.into_axis_vr(k), o2)
    })
    .build()
}
