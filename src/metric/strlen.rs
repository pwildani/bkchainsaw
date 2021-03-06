use crate::metric::Metric;
use crate::Dist;

#[derive(Default, Clone, Copy, Debug)]
pub struct StrLenMetric;

impl Metric<str> for StrLenMetric {
    #[inline]
    fn distance(&self, k1: &str, k2: &str) -> Dist {
        (k1.len() as i64 - k2.len() as i64).abs() as Dist
    }

    #[inline]
    fn distance_static(k1: &str, k2: &str) -> Dist {
        (k1.len() as i64 - k2.len() as i64).abs() as Dist
    }
}
