pub trait AsQuery<Q: ?Sized> {
    fn as_query(&self) -> &Q;
}

impl AsQuery<u64> for u64 {
    fn as_query(&self) -> &u64 {
        self
    }
}

impl AsQuery<str> for String {
    fn as_query(&self) -> &str {
        self.as_str()
    }
}
