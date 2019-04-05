pub mod hamming;

pub trait Metric<D, K> {
    fn distance(&self, k1: &K, k2: &K) -> D;
}
