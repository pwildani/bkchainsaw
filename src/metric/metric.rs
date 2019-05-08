use crate::Dist;

pub trait Metric {
    type Query: ?Sized;

    fn distance(k1: &Self::Query, k2: &Self::Query) -> Dist;
}
