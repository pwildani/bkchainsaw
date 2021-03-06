use crate::Dist;

pub trait Metric<K: ?Sized> {
    fn distance(&self, k1: &K, k2: &K) -> Dist;
    fn distance_static(k1: &K, k2: &K) -> Dist;
}
