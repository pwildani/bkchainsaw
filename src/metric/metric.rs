pub trait Metric<D, K: ?Sized> {
    fn distance(&self, k1: &K, k2: &K) -> D;
}
