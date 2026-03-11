// Claude Code Generated

use crate::positions::ChunkPos;
use std::cmp::Ordering;

/// A KD-Tree implementation specifically for ChunkPos using Manhattan distance.
/// Supports removal after build (but not insertion).
#[derive(Debug)]
pub struct ChunkPosKdTree {
    root: Option<Box<Node>>,
}

#[derive(Debug)]
enum Node {
    Internal {
        point: ChunkPos,
        split_dim: usize, // 0 for x, 1 for z
        split_value: i32,
        left: Option<Box<Node>>,
        right: Option<Box<Node>>,
        deleted: bool,
    },
    Leaf {
        point: ChunkPos,
        deleted: bool,
    },
}

impl Node {
    fn point(&self) -> ChunkPos {
        match self {
            Node::Internal { point, .. } => *point,
            Node::Leaf { point, .. } => *point,
        }
    }

    fn is_deleted(&self) -> bool {
        match self {
            Node::Internal { deleted, .. } => *deleted,
            Node::Leaf { deleted, .. } => *deleted,
        }
    }

    fn mark_deleted(&mut self) {
        match self {
            Node::Internal { deleted, .. } => *deleted = true,
            Node::Leaf { deleted, .. } => *deleted = true,
        }
    }
}

/// Manhattan distance between two ChunkPos points
#[inline]
fn manhattan_distance(a: &ChunkPos, b: &ChunkPos) -> u32 {
    a.x.abs_diff(b.x) + a.z.abs_diff(b.z)
}

impl ChunkPosKdTree {
    /// Build a new KD-Tree from a vector of ChunkPos.
    pub fn build(mut points: Vec<ChunkPos>) -> Self {
        let root = if points.is_empty() {
            None
        } else {
            Some(Box::new(Self::build_recursive(&mut points, 0)))
        };
        ChunkPosKdTree { root }
    }

    fn build_recursive(points: &mut [ChunkPos], depth: usize) -> Node {
        if points.is_empty() {
            panic!("Cannot build node from empty slice");
        }

        if points.len() == 1 {
            return Node::Leaf {
                point: points[0],
                deleted: false,
            };
        }

        let split_dim = depth % 2; // Alternate between x (0) and z (1)

        // Sort by the current dimension
        points.sort_unstable_by_key(|p| match split_dim {
            0 => p.x,
            1 => p.z,
            _ => unreachable!(),
        });

        let median_idx = points.len() / 2;
        let point = points[median_idx];
        let split_value = match split_dim {
            0 => point.x,
            1 => point.z,
            _ => unreachable!(),
        };

        let left = if median_idx > 0 {
            Some(Box::new(Self::build_recursive(
                &mut points[..median_idx],
                depth + 1,
            )))
        } else {
            None
        };

        let right = if median_idx + 1 < points.len() {
            Some(Box::new(Self::build_recursive(
                &mut points[median_idx + 1..],
                depth + 1,
            )))
        } else {
            None
        };

        Node::Internal {
            point,
            split_dim,
            split_value,
            left,
            right,
            deleted: false,
        }
    }

    /// Find the k nearest neighbors to a query point using Manhattan distance.
    /// Returns up to k points, sorted by distance (nearest first).
    pub fn nearest(&self, query: &ChunkPos, k: usize) -> Vec<ChunkPos> {
        if k == 0 {
            return Vec::new();
        }

        let mut heap = std::collections::BinaryHeap::new();

        if let Some(ref root) = self.root {
            self.nearest_recursive(root, query, k, &mut heap);
        }

        // Convert heap to sorted vector (nearest first)
        let mut entries: Vec<_> = heap.into_iter().collect();
        entries.sort_by_key(|entry| entry.dist);
        entries.into_iter().map(|entry| entry.point).collect()
    }

    fn nearest_recursive(
        &self,
        node: &Node,
        query: &ChunkPos,
        k: usize,
        heap: &mut std::collections::BinaryHeap<HeapEntry>,
    ) {
        if !node.is_deleted() {
            let point = node.point();
            let dist = manhattan_distance(query, &point);

            if heap.len() < k {
                heap.push(HeapEntry { point, dist });
            } else if let Some(worst) = heap.peek() {
                if dist < worst.dist {
                    heap.pop();
                    heap.push(HeapEntry { point, dist });
                }
            }
        }

        if let Node::Internal {
            split_dim,
            split_value,
            left,
            right,
            ..
        } = node
        {
            let query_value = match split_dim {
                0 => query.x,
                1 => query.z,
                _ => unreachable!(),
            };

            let (near, far) = if query_value <= *split_value {
                (left, right)
            } else {
                (right, left)
            };

            // Search near side first
            if let Some(ref near_node) = near {
                self.nearest_recursive(near_node, query, k, heap);
            }

            // Check if we need to search the far side
            // For Manhattan distance, the minimum possible distance to the far side
            // is the perpendicular distance to the splitting plane
            let plane_dist = query_value.abs_diff(*split_value);

            let should_search_far = if heap.len() < k {
                true
            } else if let Some(worst) = heap.peek() {
                plane_dist < worst.dist
            } else {
                true
            };

            if should_search_far {
                if let Some(ref far_node) = far {
                    self.nearest_recursive(far_node, query, k, heap);
                }
            }
        }
    }

    /// Remove a point from the tree. The point must match exactly.
    /// Returns true if the point was found and marked as deleted.
    pub fn remove(&mut self, point: &ChunkPos) -> bool {
        if let Some(ref mut root) = self.root {
            Self::remove_recursive(root, point)
        } else {
            false
        }
    }

    fn remove_recursive(node: &mut Node, point: &ChunkPos) -> bool {
        if node.point() == *point && !node.is_deleted() {
            node.mark_deleted();
            return true;
        }

        if let Node::Internal {
            split_dim,
            split_value,
            left,
            right,
            ..
        } = node
        {
            let point_value = match split_dim {
                0 => point.x,
                1 => point.z,
                _ => unreachable!(),
            };

            // Determine which side to search first
            let search_left_first = point_value <= *split_value;

            if search_left_first {
                // Try left first
                if let Some(ref mut left_node) = left {
                    if Self::remove_recursive(left_node, point) {
                        return true;
                    }
                }
                // Then try right
                if let Some(ref mut right_node) = right {
                    if Self::remove_recursive(right_node, point) {
                        return true;
                    }
                }
            } else {
                // Try right first
                if let Some(ref mut right_node) = right {
                    if Self::remove_recursive(right_node, point) {
                        return true;
                    }
                }
                // Then try left
                if let Some(ref mut left_node) = left {
                    if Self::remove_recursive(left_node, point) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Count total nodes (including deleted ones)
    pub fn count_all(&self) -> usize {
        if let Some(ref root) = self.root {
            Self::count_recursive(root)
        } else {
            0
        }
    }

    fn count_recursive(node: &Node) -> usize {
        let mut count = 1;
        if let Node::Internal { left, right, .. } = node {
            if let Some(ref left_node) = left {
                count += Self::count_recursive(left_node);
            }
            if let Some(ref right_node) = right {
                count += Self::count_recursive(right_node);
            }
        }
        count
    }

    /// Count active (non-deleted) nodes
    pub fn count_active(&self) -> usize {
        if let Some(ref root) = self.root {
            Self::count_active_recursive(root)
        } else {
            0
        }
    }

    fn count_active_recursive(node: &Node) -> usize {
        let mut count = if node.is_deleted() { 0 } else { 1 };
        if let Node::Internal { left, right, .. } = node {
            if let Some(ref left_node) = left {
                count += Self::count_active_recursive(left_node);
            }
            if let Some(ref right_node) = right {
                count += Self::count_active_recursive(right_node);
            }
        }
        count
    }
}

#[derive(Debug, Eq, PartialEq)]
struct HeapEntry {
    point: ChunkPos,
    dist: u32,
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Max heap based on distance
        self.dist.cmp(&other.dist)
    }
}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tree() {
        let tree = ChunkPosKdTree::build(vec![]);
        let results = tree.nearest(&ChunkPos::new(0, 0), 5);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_single_point() {
        let tree = ChunkPosKdTree::build(vec![ChunkPos::new(5, 10)]);
        let results = tree.nearest(&ChunkPos::new(0, 0), 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], ChunkPos::new(5, 10));
    }

    #[test]
    fn test_nearest_basic() {
        let points = vec![
            ChunkPos::new(0, 0),
            ChunkPos::new(1, 0),
            ChunkPos::new(0, 1),
            ChunkPos::new(5, 5),
            ChunkPos::new(10, 10),
        ];
        let tree = ChunkPosKdTree::build(points);

        let results = tree.nearest(&ChunkPos::new(0, 0), 3);
        assert_eq!(results.len(), 3);

        // Verify distances are correct
        assert_eq!(manhattan_distance(&results[0], &ChunkPos::new(0, 0)), 0);
        assert_eq!(manhattan_distance(&results[1], &ChunkPos::new(0, 0)), 1);
        assert_eq!(manhattan_distance(&results[2], &ChunkPos::new(0, 0)), 1);
    }

    #[test]
    fn test_nearest_sorted_by_distance() {
        let points = vec![
            ChunkPos::new(10, 10),
            ChunkPos::new(5, 5),
            ChunkPos::new(0, 0),
            ChunkPos::new(1, 1),
        ];
        let tree = ChunkPosKdTree::build(points);

        let query = ChunkPos::new(0, 0);
        let results = tree.nearest(&query, 4);

        // Verify results are sorted by distance
        for i in 0..results.len() - 1 {
            let dist_i = manhattan_distance(&results[i], &query);
            let dist_next = manhattan_distance(&results[i + 1], &query);
            assert!(dist_i <= dist_next);
        }
    }

    #[test]
    fn test_manhattan_distance_vs_euclidean() {
        let points = vec![
            ChunkPos::new(3, 0),  // Manhattan: 3, Euclidean: 3
            ChunkPos::new(2, 2),  // Manhattan: 4, Euclidean: 2.83
        ];
        let tree = ChunkPosKdTree::build(points);

        // Query from origin
        let results = tree.nearest(&ChunkPos::new(0, 0), 2);

        // With Manhattan distance, (3, 0) should be closer than (2, 2)
        assert_eq!(results[0], ChunkPos::new(3, 0));
        assert_eq!(results[1], ChunkPos::new(2, 2));
    }

    #[test]
    fn test_removal_basic() {
        let points = vec![
            ChunkPos::new(0, 0),
            ChunkPos::new(1, 0),
            ChunkPos::new(0, 1),
        ];
        let mut tree = ChunkPosKdTree::build(points);

        assert_eq!(tree.count_active(), 3);

        let removed = tree.remove(&ChunkPos::new(1, 0));
        assert!(removed);
        assert_eq!(tree.count_active(), 2);

        let results = tree.nearest(&ChunkPos::new(1, 0), 3);
        assert_eq!(results.len(), 2);
        assert!(!results.contains(&ChunkPos::new(1, 0)));
    }

    #[test]
    fn test_removal_not_found() {
        let points = vec![ChunkPos::new(0, 0)];
        let mut tree = ChunkPosKdTree::build(points);

        let removed = tree.remove(&ChunkPos::new(5, 5));
        assert!(!removed);
        assert_eq!(tree.count_active(), 1);
    }

    #[test]
    fn test_removal_all_points() {
        let points = vec![
            ChunkPos::new(0, 0),
            ChunkPos::new(1, 0),
            ChunkPos::new(0, 1),
        ];
        let mut tree = ChunkPosKdTree::build(points.clone());

        for point in &points {
            tree.remove(point);
        }

        assert_eq!(tree.count_active(), 0);
        assert_eq!(tree.count_all(), 3);

        let results = tree.nearest(&ChunkPos::new(0, 0), 5);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_k_larger_than_set() {
        let points = vec![
            ChunkPos::new(0, 0),
            ChunkPos::new(1, 0),
        ];
        let tree = ChunkPosKdTree::build(points);

        let results = tree.nearest(&ChunkPos::new(0, 0), 100);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_k_zero() {
        let tree = ChunkPosKdTree::build(vec![ChunkPos::new(0, 0)]);
        let results = tree.nearest(&ChunkPos::new(0, 0), 0);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_correctness_large_dataset() {
        // Generate a grid of points
        let mut points = Vec::new();
        for x in -10..=10 {
            for z in -10..=10 {
                points.push(ChunkPos::new(x, z));
            }
        }
        let tree = ChunkPosKdTree::build(points.clone());

        // Test several queries
        let queries = vec![
            ChunkPos::new(0, 0),
            ChunkPos::new(5, 5),
            ChunkPos::new(-7, 3),
        ];

        for query in queries {
            let k = 10;
            let results = tree.nearest(&query, k);

            // Brute force: compute all distances and find k nearest
            let mut brute_force: Vec<_> = points
                .iter()
                .map(|p| (*p, manhattan_distance(&query, p)))
                .collect();
            brute_force.sort_by_key(|(_, dist)| *dist);
            let expected: Vec<_> = brute_force.iter().take(k).map(|(p, _)| *p).collect();

            assert_eq!(results.len(), expected.len());
            for i in 0..results.len() {
                let result_dist = manhattan_distance(&query, &results[i]);
                let expected_dist = manhattan_distance(&query, &expected[i]);
                assert_eq!(result_dist, expected_dist);
            }
        }
    }

    #[test]
    fn test_stress_with_removals() {
        // Build a large tree
        let mut points = Vec::new();
        for x in 0..50 {
            for z in 0..50 {
                points.push(ChunkPos::new(x, z));
            }
        }
        let mut tree = ChunkPosKdTree::build(points.clone());
        assert_eq!(tree.count_active(), 2500);

        // Remove every other point
        for (i, point) in points.iter().enumerate() {
            if i % 2 == 0 {
                tree.remove(point);
            }
        }
        assert_eq!(tree.count_active(), 1250);

        // Verify searches still work correctly
        let query = ChunkPos::new(25, 25);
        let results = tree.nearest(&query, 20);
        assert_eq!(results.len(), 20);

        // Verify no deleted points in results
        for result in &results {
            let idx = points.iter().position(|p| p == result).unwrap();
            assert!(idx % 2 == 1, "Found a deleted point in results");
        }

        // Verify results are sorted by distance
        for i in 0..results.len() - 1 {
            let dist_i = manhattan_distance(&results[i], &query);
            let dist_next = manhattan_distance(&results[i + 1], &query);
            assert!(dist_i <= dist_next);
        }
    }

    #[test]
    fn test_negative_coordinates() {
        let points = vec![
            ChunkPos::new(-5, -5),
            ChunkPos::new(-1, -1),
            ChunkPos::new(0, 0),
            ChunkPos::new(5, 5),
        ];
        let tree = ChunkPosKdTree::build(points);

        let results = tree.nearest(&ChunkPos::new(-2, -2), 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], ChunkPos::new(-1, -1));
    }

    #[test]
    fn test_duplicate_removal_attempts() {
        let points = vec![ChunkPos::new(0, 0), ChunkPos::new(1, 1)];
        let mut tree = ChunkPosKdTree::build(points);

        assert!(tree.remove(&ChunkPos::new(0, 0)));
        assert!(!tree.remove(&ChunkPos::new(0, 0))); // Second removal should fail
        assert_eq!(tree.count_active(), 1);
    }
}
