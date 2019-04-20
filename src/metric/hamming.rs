use std::marker::PhantomData;
use std::ops::BitXor;

use crate::metric::Metric;
use crate::Dist;

pub trait CountOnes {
    #[inline]
    fn count_ones(self) -> u32;
}
impl CountOnes for u8 {
    #[inline]
    fn count_ones(self) -> u32 {
        self.count_ones()
    }
}
impl CountOnes for u16 {
    #[inline]
    fn count_ones(self) -> u32 {
        self.count_ones()
    }
}
impl CountOnes for u32 {
    #[inline]
    fn count_ones(self) -> u32 {
        self.count_ones()
    }
}
impl CountOnes for u64 {
    #[inline]
    fn count_ones(self) -> u32 {
        self.count_ones()
    }
}
impl CountOnes for u128 {
    #[inline]
    fn count_ones(self) -> u32 {
        self.count_ones()
    }
}

#[derive(Default, Clone, Copy, Derivative)]
#[derivative(Debug)]
pub struct HammingMetric<I>(#[derivative(Debug = "ignore")] PhantomData<I>)
where
    I: BitXor<I>,
    <I as BitXor<I>>::Output: CountOnes;

impl<I> Metric<I> for HammingMetric<I>
where
    I: Copy + BitXor<I>,
    <I as BitXor<I>>::Output: CountOnes,
{
    #[inline]
    fn distance(&self, k1: &I, k2: &I) -> Dist {
        (*k1 ^ *k2).count_ones() as usize
    }

    #[inline]
    fn distance_static(k1: &I, k2: &I) -> Dist {
        (*k1 ^ *k2).count_ones() as usize
    }
}
// TODO: figure out how to declare a HammingMetric over Clone and over BitXor<&I> that doesn't conflict with the above
// implementation for Copy. (The code difference is k1.clone() instead of *k1 for Clone and no
// deref for BitXor<&I>). Better yet, handle a constraint that means <&I as BitXor<&I>>::Output: CountOnes.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hamming_distance() {
        let metric: HammingMetric<u64> = Default::default();
        assert_eq!(0usize, metric.distance(&0u64, &0u64));
        assert_eq!(0usize, metric.distance(&1u64, &1u64));
        assert_eq!(1usize, metric.distance(&1u64, &0u64));
        assert_eq!(1usize, metric.distance(&0u64, &1u64));
        assert_eq!(2usize, metric.distance(&1u64, &2u64));
        assert_eq!(1usize, metric.distance(&0u64, &2u64));
    }
}
