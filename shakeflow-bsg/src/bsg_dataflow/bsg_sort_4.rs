//! Sorting network implementation.
//!
//! Currently, it implements a stable 4-item sort very efficiently.

use shakeflow::*;
use shakeflow_std::*;

use super::bsg_compare_and_swap as cas;

pub type C<Width: Num> = UniChannel<Array<Bits<Width>, U<4>>>;

pub fn m<Width: Num>() -> Module<C<Width>, C<Width>> {
    composite::<C<Width>, C<Width>, _>("bsg_sort_4", Some("i"), Some("o"), |input, k| {
        // stage 1: compare_and_swap <3,2> and <1,0>
        let cas_o0 = input
            .clone()
            .map(k, |input| cas::IProj { data: input.clip_const::<U<2>>(0), swap_on_equal: false.into() }.into())
            .comb_inline(k, cas::m::<Width, false>());

        let cas_o1 = input
            .map(k, |input| cas::IProj { data: input.clip_const::<U<2>>(2), swap_on_equal: false.into() }.into())
            .comb_inline(k, cas::m::<Width, false>());

        let s1 = cas_o0.zip(k, cas_o1).map(k, |o01| {
            let (o0, o1) = *o01;
            o0.data.append(o1.data)
        });

        // stage 2: compare_and_swap <2,0> and <3,1>
        let cas_o2 = s1
            .clone()
            .map(k, |input| cas::IProj { data: [input[0], input[2]].into(), swap_on_equal: false.into() }.into())
            .comb_inline(k, cas::m::<Width, false>());

        let cas_o3 = s1
            .map(k, |input| cas::IProj { data: [input[1], input[3]].into(), swap_on_equal: false.into() }.into())
            .comb_inline(k, cas::m::<Width, false>());

        let s2 = cas_o2.zip(k, cas_o3.clone()).map(k, |o23| {
            let (o2, o3) = *o23;
            let o2 = o2.data;
            let o3 = o3.data;

            [o2[0], o3[0], o2[1], o3[1]].into()
        });
        let swapped_3_1 = cas_o3.map(k, |o3| o3.swapped);

        // stage 3: compare_and_swap <2, 1>
        //
        // we also swap if they are equal and if <3,1> resulted in a swap
        // this will reintroduce stability into the sort
        let cas_o4 = s2
            .clone()
            .zip(k, swapped_3_1)
            .map(k, |input| {
                let (input, swapped_3_1) = *input;
                cas::IProj { data: [input[1], input[2]].into(), swap_on_equal: swapped_3_1 }.into()
            })
            .comb_inline(k, cas::m::<Width, true>());

        s2.zip(k, cas_o4).map(k, |s2o4| {
            let (s2, o4) = *s2o4;
            let o4 = o4.data;
            [s2[0], o4[0], o4[1], s2[3]].into()
        })
    })
    .build()
}
