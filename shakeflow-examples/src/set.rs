use shakeflow::*;
use shakeflow_std::*;

#[derive(Debug, Clone, Signal)]
pub struct A<const K: usize> {
    flag: bool,
    b: B<K>,
    array: Array<B<K>, U<10>>,
}

#[derive(Debug, Clone, Signal)]
pub struct B<const K: usize> {
    k_bitvec: Bits<U<K>>,
}

pub fn m<const K: usize>() -> Module<UniChannel<(A<K>, B<K>)>, UniChannel<A<K>>> {
    composite::<UniChannel<(A<K>, B<K>)>, UniChannel<A<K>>, _>("setter", Some("input"), Some("output"), |value, k| {
        value.map(k, |ab| {
            let (a, b) = *ab;
            a.set_flag(false.into()).set_b(b.set_k_bitvec(10.into())).set_array(b.repeat())
        })
    })
    .build()
}
