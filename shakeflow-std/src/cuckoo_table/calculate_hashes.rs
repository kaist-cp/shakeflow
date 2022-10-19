use std::array;

use shakeflow::*;

use super::types::*;
use super::{HashType, KeyType};
use crate::*;

/// Calculate NUM_TABLES hashes given a key.
pub(super) fn m() -> Module<VrChannel<KeyType>, [VrChannel<HashType, { Protocol::Demanding }>; NUM_TABLES]> {
    composite::<VrChannel<KeyType>, [VrChannel<HashType, { Protocol::Demanding }>; NUM_TABLES], _>(
        "calculate_hashes",
        Some("in"),
        Some("out"),
        |input, k| {
            input.duplicate_n::<NUM_TABLES>(k).array_enumerate().map(|i, ch| {
                ch.map(k, move |key| {
                    Expr::from(array::from_fn(|k| {
                        Expr::from((
                            Expr::from(TABULATION_TABLE[i][1][k] & ((1 << TABLE_ADDRESS_BITS) - 1)),
                            (TABULATION_TABLE[i][0][k] & ((1 << TABLE_ADDRESS_BITS) - 1)).into(),
                        ))
                    }))
                    .zip(key)
                    .map(|x| {
                        let (value_tuple, key_kth) = *x;
                        key_kth.cond(value_tuple.1, value_tuple.0)
                    })
                    .tree_fold(|lhs, rhs| lhs ^ rhs)
                })
            })
        },
    )
    .build()
}
