//! Utilities.

use std::iter::IntoIterator;

use arrayvec::ArrayVec;

/// Returns ceiling log2.
pub const fn clog2(value: usize) -> usize {
    if value == 0 {
        0
    } else {
        (::std::mem::size_of::<usize>() * 8) - (value - 1).leading_zeros() as usize
    }
}

/// Returns floor log2
pub const fn flog2(val: usize) -> usize {
    if val == 1 {
        0
    } else {
        1 + flog2(val >> 1)
    }
}

/// Return aligned value of `value` by `by`
///
/// ### Example
/// ```ignore
/// let value = 15;
/// let byte_aligned = align_usize(value, 8);
/// assert_eq!(byte_aligned, 16);
/// ````
pub const fn align_usize(value: usize, by: usize) -> usize { (value + by - 1) / by * by }

/// Returns bit-represented value of an integer.
pub fn usize_to_bitvec(n: usize, value: usize) -> Vec<bool> {
    assert!(n >= clog2(value + 1), "Width of Expr ({}) is too small to be converted from the value '{}'", n, value);
    let size_of_usize = ::std::mem::size_of::<usize>();
    (0..n).map(|i| if i >= size_of_usize * 8 { false } else { (value & (1 << i)) != 0 }).collect::<Vec<_>>()
}

/// Returns bit-represented value of an integer.
// TODO: Make this function `const fn`.
pub fn usize_to_bits<const N: usize>(value: usize) -> [bool; N] { usize_to_bitvec(N, value).try_into().unwrap() }

/// Returns bit-represented value of an integer.
// TODO: Make this function `const fn`.
pub fn u32_to_bits<const N: usize>(value: u32) -> [bool; N] {
    let size_of_u32 = ::std::mem::size_of::<u32>();
    (0..N)
        .map(|i| if i >= size_of_u32 * 8 { false } else { (value & (1 << i)) != 0 })
        .collect::<ArrayVec<bool, N>>()
        .into_inner()
        .unwrap()
}

/// Returns bit-represented value of an integer.
// TODO: Make this function `const fn`.
pub fn u64_to_bits<const N: usize>(value: u64) -> [bool; N] {
    let size_of_u64 = ::std::mem::size_of::<u64>();
    (0..N)
        .map(|i| if i >= size_of_u64 * 8 { false } else { (value & (1 << i)) != 0 })
        .collect::<ArrayVec<bool, N>>()
        .into_inner()
        .unwrap()
}

/// Combines all elements into one String, separated by `sep`. Returns `None` if all elements are `None`.
// TODO: Make this function macro
pub fn join_options<I>(sep: &str, iterable: I) -> Option<String>
where I: IntoIterator<Item = Option<String>> {
    let iterable = iterable.into_iter().flatten().collect::<Vec<_>>();
    if iterable.is_empty() {
        None
    } else {
        Some(iterable.join(sep))
    }
}

/// Indents every line in the string.
pub fn indent(str: String, indent: usize) -> String {
    str.lines().map(|l| format!("{}{}", " ".repeat(indent), l)).collect::<Vec<_>>().join("\n")
}

/// Ok or executing the given expression.
#[macro_export]
macro_rules! ok_or {
    ($e:expr, $err:expr) => {{
        match $e {
            Ok(r) => r,
            Err(_) => $err,
        }
    }};
}

/// Some or executing the given expression.
#[macro_export]
macro_rules! some_or {
    ($e:expr, $err:expr) => {{
        match $e {
            Some(r) => r,
            None => $err,
        }
    }};
}
