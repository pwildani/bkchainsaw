use crate::metric::Metric;
use crate::Dist;

pub trait KeyQuery: Default {
    type Key: Clone;
    type Query: ?Sized;

    fn distance<M: Metric<Self::Query>>(
        &self,
        metric: &M,
        key: &Self::Key,
        query: &Self::Query,
    ) -> Dist;
    fn to_key(&self, query: &Self::Query) -> Self::Key;
    fn eq(&self, key: &Self::Key, query: &Self::Query) -> bool;
}
