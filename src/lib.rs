mod bknode;
mod bktree;
mod metric;

pub use bknode::BkNode;
pub use bktree::BkInRamAllocator;
pub use bktree::BkInRamTree;
pub use bktree::BkTree;
pub use metric::hamming::HammingMetric;

#[macro_use]
extern crate derivative;

#[cfg(test)]
mod tests {
    use crate::bktree::BkInRamTree;
    use crate::bktree::BkTree;
    use crate::bktree::StringKey;
    use crate::bktree::U64Key;
    use crate::metric::hamming::HammingMetric;
    use crate::metric::strlen::StrLenMetric;

    fn hamming_tree() -> BkInRamTree<U64Key, HammingMetric<u64>> {
        BkTree::new(Default::default())
        //let metric: HammingMetric<u64> = Default::default();
        //let mut tree : BkInRamTree<u64, HammingMetric<u64>, u64> = BkTree::new(metric);
        //tree
    }

    #[test]
    fn can_construct_empty_tree() {
        let mut tree = hamming_tree();
        println!("Empty Tree: {:?}", tree)
    }

    #[test]
    fn can_add_one_value() {
        let mut tree = hamming_tree();
        tree.add(&0u64);
        println!("Zero Tree: {:?}", tree)
    }

    #[test]
    fn can_add_repeated_roots() {
        let mut tree = hamming_tree();
        tree.add(&0u64);
        tree.add(&0u64);
        tree.add(&0u64);
        println!("Zeros Tree: {:?}", tree)
    }

    #[test]
    fn can_add_repeated_children() {
        let mut tree = hamming_tree();
        tree.add(&0u64);
        tree.add(&1u64);
        tree.add(&1u64);
        tree.add(&1u64);
        println!("Ones Tree: {:?}", tree)
    }

    #[test]
    fn can_add_distinct_values() {
        let mut tree = hamming_tree();
        tree.add(&0u64);
        tree.add(&1u64);
        tree.add(&2u64);
        tree.add(&3u64);
        println!("Many Tree: {:?}", tree)
    }

    #[test]
    fn can_add_distinct_values_in_reverse() {
        let mut tree = hamming_tree();
        tree.add(&3u64);
        tree.add(&2u64);
        tree.add(&1u64);
        tree.add(&0u64);
        println!("Many Tree Reversed: {:?}", tree)
    }

    #[test]
    fn can_construct_empty_string_tree() {
        let mut tree: BkInRamTree<StringKey, StrLenMetric> = BkTree::new(StrLenMetric);
        println!("Empty string tree: {:?}", tree);
    }

    #[test]
    fn can_add_empty_string() {
        let mut tree: BkInRamTree<StringKey, StrLenMetric> = BkTree::new(StrLenMetric);
        tree.add("");
        println!("empty string tree: {:?}", tree);
    }

    #[test]
    fn can_add_string() {
        let mut tree: BkInRamTree<StringKey, StrLenMetric> = BkTree::new(StrLenMetric);
        tree.add("foo");
        println!("foo string tree: {:?}", tree);
    }

    #[test]
    fn can_add_many_strings() {
        let mut tree: BkInRamTree<StringKey, StrLenMetric> = BkTree::new(StrLenMetric);
        tree.add("foo");
        tree.add("bar");
        tree.add("baz");
        tree.add("left");
        tree.add("ship");
        println!("many string tree: {:?}", tree);
    }
}
