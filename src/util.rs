//! Helper utilities
use std::ops::AddAssign;

use num::Integer;

/// Converts an `Iterator` over any integral primitive type into `SetVariationIterator`,
/// which will enumerate every variation of the numbers in the list. This is blanket implemented
/// over every fixed-sized iterator with integers, but given how many variations are generated
/// (more than a power set, so think exponential as a floor) you ideally do not want more than
/// an extremely small number of elements, whose values are all extremely small.
pub trait SetEnumerator<N>: ExactSizeIterator<Item = N>
where
    N: Integer,
{
    /// Adapts the given `ExactSizeIterator` to a `SetVariationIterator`.
    #[inline]
    fn possibilities(self) -> SetVariationIterator<N>
    where
        Self: Sized,
    {
        SetVariationIterator {
            maxes: self.collect(),
            variation: None,
            finished: false,
        }
    }
}

impl<N, I> SetEnumerator<N> for I
where
    N: Integer,
    I: ExactSizeIterator<Item = N>,
{
}

/// Enumerates variations of the given set of numbers. To my knowledge there's no clean technical term for what
/// precisely this does, it's more complex than a power set due to operating on arbitrary integers, but it's not
/// exactly combinations or permutations either, since some objects are treated as mutually exclusive (ones sharing the
/// same "slot").
///
/// The best analogy is that this generates every possible variant of a number, where each digit can have a different base.
/// In fact, if you create this from a two-element iterator whose values are `5` and `9`, you'll end up with the values
/// `0,0`-`5,9` or all possible minutes before they roll over to the hour. If any input value is zero, it is essentially treated
/// as an empty space and will always be zero, if any value is negative, it will be treated as zero.
pub struct SetVariationIterator<N>
where
    N: Integer,
{
    /// The digits to generate variants for, where each digit has a base equal to the value
    /// in that slot.
    maxes: Vec<N>,
    /// The current variation, `None` at the beginning.
    variation: Option<Vec<N>>,
    /// Whether this iterator has generated every variant. If it has it yields `None`.
    finished: bool,
}

impl<'a, N> Iterator for SetVariationIterator<N>
where
    N: Integer + AddAssign + Clone + Copy,
{
    type Item = Vec<N>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished || self.maxes.is_empty() {
            return None;
        }

        // Beginner note: ATM due to limitations with associated constants and such, we need to use
        // a crate called `Num` to get zero values for genericity over integers, which lets us get values
        // like `zero` and `one` via trait, so we need to use them instead of `0` and `1`.
        match self.variation {
            None => {
                let variation = vec![N::zero(); self.maxes.len()];
                self.variation = Some(variation.clone());
                Some(variation)
            }
            Some(ref mut variation) => {
                variation[0] += N::one();
                for (idx, max) in self.maxes.iter().enumerate() {
                    if variation[idx] > *max {
                        variation[idx] = N::zero();
                        if idx < variation.len() - 1 {
                            variation[idx + 1] += N::one();
                        } else {
                            self.finished = true;
                            return None;
                        }
                    } else {
                        break;
                    }
                }

                Some(variation.clone())
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::util::SetEnumerator;

    #[test]
    fn power_set() {
        let maxes = vec![3, 1, 1];

        let expected = vec![
            vec![0, 0, 0],
            vec![1, 0, 0],
            vec![2, 0, 0],
            vec![3, 0, 0],
            vec![0, 1, 0],
            vec![1, 1, 0],
            vec![2, 1, 0],
            vec![3, 1, 0],
            vec![0, 0, 1],
            vec![1, 0, 1],
            vec![2, 0, 1],
            vec![3, 0, 1],
            vec![0, 1, 1],
            vec![1, 1, 1],
            vec![2, 1, 1],
            vec![3, 1, 1],
        ];

        let result = maxes.into_iter().possibilities().collect::<Vec<_>>();
        assert_eq!(result, expected);
    }

    #[test]
    fn power_set_zero_slot() {
        let maxes = vec![2, 0, 1];

        let expected = vec![
            vec![0, 0, 0],
            vec![1, 0, 0],
            vec![2, 0, 0],
            vec![0, 0, 1],
            vec![1, 0, 1],
            vec![2, 0, 1],
        ];

        let result = maxes.into_iter().possibilities().collect::<Vec<_>>();
        assert_eq!(result, expected);
    }

    #[test]
    fn power_set_empty() {
        let maxes: Vec<i32> = vec![];

        assert_eq!(maxes.into_iter().possibilities().next(), None);
    }
}
