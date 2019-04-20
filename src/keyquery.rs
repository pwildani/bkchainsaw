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

    fn distance_static<M: Metric<Self::Query>>(
        metric: &M,
        key: &Self::Key,
        query: &Self::Query,
    ) -> Dist;

    fn to_key(&self, query: &Self::Query) -> Self::Key;
    fn eq(&self, key: &Self::Key, query: &Self::Query) -> bool;

    fn to_key_static(query: &Self::Query) -> Self::Key;
    fn eq_static(key: &Self::Key, query: &Self::Query) -> bool;

    fn to_query_static(key: &Self::Key) -> &Self::Query;
}
