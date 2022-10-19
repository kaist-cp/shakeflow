use shakeflow::*;
use shakeflow_std::*;

const WIDTH_P: usize = 10;

#[derive(Debug, Clone, Signal)]
pub struct V {
    data: Bits<U<WIDTH_P>>,
}

pub type IC = VrChannel<V>;
pub type OC = [VrChannel<V>; 2];

#[derive(Debug, Clone, Signal)]
pub struct S {
    data: Bits<U<WIDTH_P>>,
    valid: bool,
}

#[derive(Debug, Clone, Signal)]
pub struct Feedback {
    ready: Bits<U<2>>,
    filter: bool,
}

pub type FeedbackC = UniChannel<Feedback>;

pub fn m() -> Module<IC, OC> {
    // TODO: Use other combinator
    composite::<(IC, FeedbackC), (OC, FeedbackC), _>("bsg_two_buncher", Some("i"), Some("o"), |(input, feedback), k| {
        let ready = feedback.clone().map(k, |input| input.ready);
        let filter = feedback.map(k, |input| input.filter);

        let input = input.filter_bwd(k, filter).into_uni(k, true);

        let output = input.zip(k, ready).fsm_map::<S, (Array<Valid<V>, U<2>>, bool), _>(
            k,
            None,
            SProj { data: Expr::x(), valid: false.into() }.into(),
            |input, state| {
                let (input, ready_i) = *input;

                let v_i = input.valid;
                let data_i = input.inner.data;

                let data_r = state.data;
                let data_v_r = state.valid;

                let (data_v_n, data_en) = *select! {
                    (!data_v_r & v_i) => (!ready_i[0], !ready_i[0]).into(),
                    (data_v_r & ready_i[0] & v_i) => (!ready_i[1], !ready_i[1]).into(),
                    (data_v_r & ready_i[0] & !v_i) => (false, false).into(),
                    default => (data_v_r, false.into()).into(),
                };

                let v_o = (data_v_r | v_i).repr().append((data_v_r & v_i).repr());
                let data_o = Expr::from(VProj { data: data_v_r.cond(data_r, data_i) })
                    .repeat::<U<1>>()
                    .append(input.inner.repeat::<U<1>>());
                let ready_o = (ready_i[0] & v_i) | !data_v_r;

                let output = Expr::<Valid<_>>::new_arr(v_o, data_o).resize();
                let s_next = SProj { data: data_en.cond(data_i, data_r), valid: data_v_n }.into();

                ((output, ready_o).into(), s_next)
            },
        );

        let filter = output.clone().map(k, |input| input.1);
        let (output, ready) = output
            .map(k, |input| input.0)
            .slice(k)
            .array_map(k, "output", |out, k| {
                let out = out.into_vr(k);
                let (out, ready) = out.fire(k);
                (out, ready)
            })
            .unzip();

        let feedback =
            ready.concat(k).zip(k, filter).map(k, |input| FeedbackProj { ready: input.0, filter: input.1 }.into());

        (output, feedback)
    })
    .loop_feedback()
    .build()
}
