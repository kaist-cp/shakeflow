use shakeflow::*;
use shakeflow_std::*;

pub fn m() -> Module<UniChannel<bool>, UniChannel<bool>> {
    composite::<UniChannel<bool>, UniChannel<bool>, _>("if_bug_test", Some("i"), Some("o"), |i, k| {
        i.map(k, |_| {
            let op1 = Expr::from(true).cond(true.into(), false.into());
            let op2 = Expr::from(false).cond(false.into(), true.into());

            Expr::from(true).cond(op1, op2)
        })
    })
    .build()
}
