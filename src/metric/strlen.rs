use crate::metric::Metric;

#[derive(Default, Clone, Copy, Debug)]
pub struct StrLenMetric;

impl<Dist> Metric<Dist, str> for StrLenMetric
where
    Dist: From<u64>,
{
    fn distance(&self, k1: &str, k2: &str) -> Dist {
        ((k1.len() as i64 - k2.len() as i64).abs() as u64).into()
    }
}
