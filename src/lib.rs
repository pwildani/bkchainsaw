mod bknode;
mod bktree;
mod metric;

pub use bknode::BkNode;
pub use bktree::BkInRamAllocator;
pub use bktree::BkTree;
pub use metric::hamming::HammingMetric;

#[macro_use]
extern crate derivative;

#[cfg(test)]
mod tests {
    use crate::bktree::BkTree;
    use crate::metric::hamming::HammingMetric;

    #[test]
    fn can_construct_empty_tree() {
        let metric: HammingMetric<u64> = Default::default();
        let tree = BkTree::new(metric);
        println!("Empty Tree: {:?}", tree)
    }

    #[test]
    fn can_add_one_value() {
        let metric: HammingMetric<u64> = Default::default();
        let mut tree = BkTree::new(metric);
        tree.add(0u64);
        println!("Zero Tree: {:?}", tree)
    }

    #[test]
    fn can_add_repeated_roots() {
        let metric: HammingMetric<u64> = Default::default();
        let mut tree = BkTree::new(metric);
        tree.add(0u64);
        tree.add(0u64);
        tree.add(0u64);
        println!("Zeros Tree: {:?}", tree)
    }

    #[test]
    fn can_add_repeated_children() {
        let metric: HammingMetric<u64> = Default::default();
        let mut tree = BkTree::new(metric);
        tree.add(0u64);
        tree.add(1u64);
        tree.add(1u64);
        tree.add(1u64);
        println!("Ones Tree: {:?}", tree)
    }

    #[test]
    fn can_add_distinct_values() {
        let metric: HammingMetric<u64> = Default::default();
        let mut tree = BkTree::new(metric);
        tree.add(0u64);
        tree.add(1u64);
        tree.add(2u64);
        tree.add(3u64);
        println!("Many Tree: {:?}", tree)
    }

    #[test]
    fn can_add_distinct_values_in_reverse() {
        let metric: HammingMetric<u64> = Default::default();
        let mut tree = BkTree::new(metric);
        tree.add(3u64);
        tree.add(2u64);
        tree.add(1u64);
        tree.add(0u64);
        println!("Many Tree Reversed: {:?}", tree)
    }
}
