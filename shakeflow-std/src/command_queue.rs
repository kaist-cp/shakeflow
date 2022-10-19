//! Command queue.
//!
//! TODO: Documentation

use crate::*;

/// Hazard type.
#[derive(Debug, Clone, Signal)]
pub struct Hazard<Sel: Default + Signal, Cmd: Command, const PIPELINE: usize> {
    selector: Sel,
    targets: Array<Valid<Cmd>, U<PIPELINE>>,
}

impl<Sel: Default + Signal, Cmd: Command, const PIPELINE: usize> Hazard<Sel, Cmd, PIPELINE> {
    /// Creates new expr.
    pub fn new_expr() -> Expr<'static, Self> {
        HazardProj { selector: Sel::default().into(), targets: Expr::x() }.into()
    }

    /// Calculates pipe hazard.
    pub fn pipe_hazard<'id>(hazard: Expr<'id, Self>, target: Expr<'id, Cmd>) -> Expr<'id, bool> {
        (0..PIPELINE)
            .map(|i| {
                let stage_target = hazard.targets[i];
                stage_target.valid & Cmd::collision(stage_target.inner, target)
            })
            .fold(false.into(), |acc, stage_hazard| acc | stage_hazard)
    }
}

/// Signal type used in command queue.
pub trait Command: Signal {
    /// Returns `true` if `lhs` and `rhs` has collision, so that it cannot be pushed into command queue.
    fn collision<'id>(lhs: Expr<'id, Self>, rhs: Expr<'id, Self>) -> Expr<'id, bool>;
}

/// Command queue.
///
/// # Note
///
/// `allow_duplicate` argument represents whether there can be multiple same type commands in the queue.
pub fn m_command_queue<V: Command, const N: usize, const PIPELINE: usize>(
    allow_duplicate: [bool; N],
) -> Module<([VrChannel<V>; N], UniChannel<Bits<U<N>>>), ((UniChannel<(V, Bits<U<N>>)>, UniChannel<(V, Bits<U<N>>)>), ())>
{
    composite::<
        (([VrChannel<V>; N], UniChannel<Bits<U<N>>>), UniChannel<Hazard<Bits<U<N>>, V, PIPELINE>>),
        (((UniChannel<(V, Bits<U<N>>)>, UniChannel<(V, Bits<U<N>>)>), ()), UniChannel<Hazard<Bits<U<N>>, V, PIPELINE>>),
        _,
    >("command_queue", Some("in"), Some("out"), |((input, extra_hazard), hazard), k| {
        // 1. Filtering out stage hazard
        let allow_duplicate = UniChannel::source(k, allow_duplicate.into());
        let cond_duplicate = allow_duplicate
            .zip3(k, hazard.clone(), extra_hazard)
            .map(k, |input| {
                let (allow_duplicate, hazard, extra_hazard) = *input;
                (allow_duplicate | !hazard.selector) & extra_hazard
            })
            .slice(k);

        let input = input.array_zip(cond_duplicate).array_map_feedback(
            k,
            hazard,
            "filter_hazard",
            |((input, cond_duplicate), hazard), k| {
                input.zip_uni(k, cond_duplicate).zip_uni(k, hazard).assert_map(k, |input| {
                    let (input, hazard) = *input;
                    let (input, cond_duplicate) = *input;
                    let cond = cond_duplicate & !Hazard::pipe_hazard(hazard, input);
                    (cond, input).into()
                })
            },
        );

        // 2. Muxing input channels
        let (selected, pipeline_input) = input.mux_quick(k);
        let pipeline_input = pipeline_input.zip(k, selected);

        // 3. Pipelining
        let pipeline_stages = pipeline_input
            .clone()
            .buffer(k, (Expr::x(), 0.into()).into())
            .window::<PIPELINE>((Expr::x(), 0.into()).into(), k);

        let pipeline_stage = pipeline_stages.clone().map(k, |input| input[PIPELINE - 1]);

        let hazard = pipeline_stages.map(k, |input| {
            input.enumerate().fold(Hazard::<Bits<U<N>>, V, PIPELINE>::new_expr(), |hazard, istage| {
                let (i, stage) = *istage;

                let stage_is_active = stage.1.is_gt(0.into());
                let stage_hazard = Expr::<Valid<_>>::new(stage_is_active, stage.0);

                HazardProj::<Bits<U<N>>, V, PIPELINE> {
                    selector: hazard.selector | stage.1,
                    targets: hazard.targets.set(i, stage_hazard),
                }
                .into()
            })
        });

        (((pipeline_input, pipeline_stage), ()), hazard)
    })
    .loop_feedback()
    .build()
}

/// Extension for command queue.
pub trait CommandQueueExt<I: Interface, const N: usize> {
    /// TODO: Documentation
    type O<const PIPELINE: usize>: Interface;

    /// TODO: Documentation
    fn command_queue<const PIPELINE: usize>(
        self, k: &mut CompositeModuleContext, allow_duplicate: [bool; N],
    ) -> (Self::O<PIPELINE>, Module<UniChannel<Bits<U<N>>>, ()>);
}

impl<V: Command, const N: usize> CommandQueueExt<VrChannel<V>, N> for [VrChannel<V>; N] {
    type O<const PIPELINE: usize> = (UniChannel<(V, Bits<U<N>>)>, UniChannel<(V, Bits<U<N>>)>);

    fn command_queue<const PIPELINE: usize>(
        self, k: &mut CompositeModuleContext, allow_duplicate: [bool; N],
    ) -> (Self::O<PIPELINE>, Module<UniChannel<Bits<U<N>>>, ()>) {
        let module = k.register_inline(m_command_queue::<V, N, PIPELINE>(allow_duplicate));
        let (command_queue, extra_hazard) = module.split();
        let o = self.comb_inline(k, command_queue);
        (o, extra_hazard)
    }
}
