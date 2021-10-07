use std::ops::AddAssign;

use num::Integer;

pub trait SetEnumerator<N>: ExactSizeIterator<Item = N>
where
    N: Integer,
{
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

pub struct SetVariationIterator<N>
where
    N: Integer,
{
    maxes: Vec<N>,
    variation: Option<Vec<N>>,
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
