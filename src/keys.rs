use crate::keyquery::KeyQuery;
use crate::metric::Metric;
use crate::Dist;

#[derive(Debug, Clone, Copy, Default)]
pub struct U64Key;

impl KeyQuery for U64Key {
    type Key = u64;
    type Query = u64;

    #[inline]
    fn distance<M: Metric<Self::Query>>(
        &self,
        metric: &M,
        key: &Self::Key,
        query: &Self::Query,
    ) -> Dist {
        metric.distance(key, query)
    }

    #[inline]
    fn to_key(&self, query: &Self::Query) -> Self::Key {
        *query
    }

    #[inline]
    fn eq(&self, key: &Self::Key, query: &Self::Query) -> bool {
        key == query
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StringKey;

impl KeyQuery for StringKey {
    type Key = String;
    type Query = str;

    #[inline]
    fn distance<M: Metric<Self::Query>>(
        &self,
        metric: &M,
        key: &Self::Key,
        query: &Self::Query,
    ) -> Dist {
        metric.distance(key.as_str(), &query)
    }

    #[inline]
    fn to_key(&self, query: &Self::Query) -> String {
        query.to_string()
    }

    #[inline]
    fn eq(&self, key: &Self::Key, query: &Self::Query) -> bool {
        key.as_str() == query
    }
}
