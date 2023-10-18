/*


                                        [A+B+C+D+E+F+G+H+I+J+K+L+M+N+O]
                __________________________________/        \________________________________
            [A+B+C+D+E+F+G+H]                                      [I+J+K+L+M+N+O]
          ______/      \_____                                  ______/      \_____
     [A+B+C+D]              [E+F+G+H]                    [I+J+K+L]               [M+N+O]
     /         \            /       \                    /       \               /       \
 [A+B]         [C+D]    [E+F]      [G+H]          [I+J]       [K+L]         [M+N]       [O]
 /   \         /   \    /   \      /   \          /   \       /   \         /   \
A     B       C     D  E     F    G     H        I     J     K     L       M     N

*/
use std::collections::VecDeque;

// https://en.algorithmica.org/hpc/data-structures/binary-search#eytzinger-layout
// https://github.com/cockroachdb/pebble
// Few assumptions:
// - data provided to the index is in time ascending order.
// - data is immutable.
// - data is not sparse.
#[derive(Clone, Debug, Copy, PartialEq)]
// Span is a half-open interval [start, end)
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Default for Span {
    fn default() -> Self {
        Span { start: 0, end: 0 }
    }
}

#[derive(Clone, Debug, Copy, PartialEq)]
// ISegment is a segment of aggregations indexed by the ISegmentIndex.
pub struct ISegment {
    pub span: Span,
    pub count: usize,
    pub max: f64,
    pub min: f64,
    pub sum: f64,
}

impl Default for ISegment {
    fn default() -> Self {
        Self {
            span: Span::default(),
            count: 0,
            max: 0.,
            min: 0.,
            sum: 0.,
        }
    }
}

// ISegmentIndex is a data structure that answers aggr queries in O(log n) time.
pub struct ISegmentIndex {
    pub tree: Vec<ISegment>,
}

impl ISegmentIndex {
    pub fn new(values: Vec<ISegment>) -> Self {
        let tree_size = 2 * (2usize.pow(((values.len()) as f64).log2().ceil() as u32)) - 1;
        let mut seg_forest = Self {
            tree: vec![ISegment::default(); tree_size],
        };
        seg_forest.build(&values, 0, 0, values.len() - 1);
        seg_forest
    }

    pub fn build(&mut self, values: &[ISegment], index: usize, left: usize, right: usize) {
        if left == right {
            if left < values.len() {
                self.tree[index] = values[left];
            }
        } else {
            let mid: usize = left + (right - left) / 2;
            self.build(values, index * 2 + 1, left, mid);
            self.build(values, index * 2 + 2, mid + 1, right);

            let left_child = self.tree[index * 2 + 1];
            let right_child = self.tree[index * 2 + 2];

            self.tree[index] = combine(left_child, right_child);
        }
    }

    pub fn append(&mut self, value: ISegment) {
        let tree_size = self.tree.len();
        let mut new_value_index = (tree_size + 1) / 2;

        if new_value_index * 2 >= tree_size {
            // Double the size of the tree to accommodate the new value.
            let new_tree_size = tree_size * 2 + 1;
            self.tree.resize(new_tree_size, ISegment::default());
        }

        // Insert the new value at the appropriate leaf position.
        self.tree[new_value_index] = value;

        // Update the internal nodes.
        while new_value_index > 0 {
            new_value_index = (new_value_index - 1) / 2;

            let left_child_index = new_value_index * 2 + 1;
            let right_child_index = new_value_index * 2 + 2;

            self.tree[new_value_index] =
                combine(self.tree[left_child_index], self.tree[right_child_index])
        }
    }

    pub fn update(&mut self, target_start: usize, value: ISegment) {
        fn update_recursive(
            tree: &mut Vec<ISegment>,
            node_index: usize,
            target_start: usize,
            value: &ISegment,
        ) {
            if target_start >= tree[node_index].span.start
                && target_start <= tree[node_index].span.end
            {
                if tree[node_index].span.start == tree[node_index].span.end {
                    tree[node_index] = value.clone();
                } else {
                    let left_child_index = node_index * 2 + 1;
                    let right_child_index = node_index * 2 + 2;

                    update_recursive(tree, left_child_index, target_start, value);
                    update_recursive(tree, right_child_index, target_start, value);

                    let left_child = &tree[left_child_index];
                    let right_child = &tree[right_child_index];

                    tree[node_index] = ISegment {
                        span: Span {
                            start: left_child.span.start,
                            end: right_child.span.end,
                        },
                        count: left_child.count + right_child.count,
                        max: left_child.max.max(right_child.max),
                        min: left_child.min.min(right_child.min),
                        sum: left_child.sum + right_child.sum,
                    };
                }
            }
        }

        update_recursive(&mut self.tree, 0, target_start, &value);
    }

    pub fn print_tree(&self) {
        fn print_node_recursive(
            tree: &Vec<ISegment>,
            node_index: usize,
            depth: usize,
            is_right: bool,
        ) {
            if node_index >= tree.len() {
                return;
            }

            let left_child_index = node_index * 2 + 1;
            let right_child_index = node_index * 2 + 2;

            print_node_recursive(tree, right_child_index, depth + 1, true);

            let indent = "      ".repeat(depth);
            let branch = if is_right { " /" } else { " \\" };
            print!("{}{}", indent, branch);
            print!("----");

            print!("<{:?},{}>", node_index, tree[node_index].sum);
            println!();

            print_node_recursive(tree, left_child_index, depth + 1, false);
        }
        print_node_recursive(&self.tree, 0, 0, true);
    }

    pub fn query_bfs(&self, query_span: Span) -> Option<ISegment> {
        let mut queue: VecDeque<usize> = VecDeque::new();
        queue.push_back(0);

        let mut result: Option<ISegment> = None;

        while let Some(i) = queue.pop_front() {
            if i >= self.tree.len() {
                return result;
            }

            if query_span.end < self.tree[i].span.start || self.tree[i].span.end < query_span.start
            {
                // no overlap
                continue;
            }

            if query_span.start <= self.tree[i].span.start
                && self.tree[i].span.end <= query_span.end
            {
                // total overlap
                result = match result {
                    Some(res) => Some(ISegment {
                        span: Span {
                            start: res.span.start.min(self.tree[i].span.start),
                            end: res.span.start.max(self.tree[i].span.end),
                        },
                        count: res.count + self.tree[i].count,
                        max: res.max.max(self.tree[i].max),
                        min: res.min.min(self.tree[i].min),
                        sum: res.sum + self.tree[i].sum,
                    }),
                    None => Some(self.tree[i]),
                };
                continue;
            }
            queue.push_back(i * 2 + 1);
            queue.push_back(i * 2 + 2);
        }
        return result;
    }

    pub fn query_dfs(&self, index: usize, query_span: Span) -> Option<ISegment> {
        if index >= self.tree.len() {
            return None;
        }

        if query_span.end < self.tree[index].span.start
            || self.tree[index].span.end < query_span.start
        {
            // no overlap
            return None;
        }

        if query_span.start <= self.tree[index].span.start
            && self.tree[index].span.end <= query_span.end
        {
            // total overlap
            return Some(self.tree[index]);
        }

        let left_res = self.query_dfs(index * 2 + 1, query_span);
        let right_res = self.query_dfs(index * 2 + 2, query_span);

        match (left_res, right_res) {
            (Some(left), Some(right)) => Some(ISegment {
                span: Span {
                    start: left.span.start,
                    end: right.span.end,
                },
                count: left.count + right.count,
                max: left.max.max(right.max),
                min: left.min.min(right.min),
                sum: left.sum + right.sum,
            }),
            (Some(left), None) => Some(left),
            (None, Some(right)) => Some(right),
            (None, None) => None,
        }
    }
}

fn combine(left: ISegment, right: ISegment) -> ISegment {
    return ISegment {
        span: Span {
            start: left.span.start,
            end: right.span.end,
        },
        count: left.count + right.count,
        max: left.max.max(right.max),
        min: left.min.min(right.min),
        sum: left.sum + right.sum,
    };
}

#[cfg(test)]
mod tests {
    use super::{ISegment, ISegmentIndex, Span};

    fn tree_data() -> (Vec<ISegment>, ISegmentIndex) {
        let mut data: Vec<ISegment> = vec![ISegment::default(); 6];
        for i in 0..data.len() {
            let time: usize = i;
            let val: f64 = i as f64;
            data[i] = ISegment {
                count: 1,
                max: val,
                min: val,
                sum: val,
                span: Span {
                    start: time,
                    end: time + 1,
                },
            };
        }

        (data.clone(), ISegmentIndex::new(data))
    }

    #[test]
    fn build() {
        let (data, mut tree) = tree_data();
        tree.build(&data, 0, 0, data.len() - 1);
        for i in 0..tree.tree.len() {
            println!("{:?}", tree.tree[i]);
        }

        assert_eq!(tree.tree[0].count, 6);
    }

    #[test]
    fn sum() {
        let (data, mut tree) = tree_data();
        tree.build(&data, 0, 0, data.len() - 1);

        for i in 0..data.len() {
            print!("{:?} ", data[i].sum);
        }

        for i in 0..tree.tree.len() {
            println!("{:?}", tree.tree[i]);
        }

        assert_eq!(
            tree.query_bfs(Span { start: 1, end: 6 },).unwrap().sum,
            15.0,
        );

        assert_eq!(
            tree.query_bfs(Span { start: 1, end: 6 },).unwrap().sum,
            15.0,
        );

        assert_eq!(tree.query_bfs(Span { start: 1, end: 3 },).unwrap().sum, 3.0);

        assert_eq!(
            tree.query_bfs(Span { start: 0, end: 6 },).unwrap().sum,
            15.0
        );
    }

    #[test]
    fn max() {
        let (data, mut tree) = tree_data();
        tree.build(&data, 0, 0, data.len() - 1);

        assert_eq!(tree.query_bfs(Span { start: 2, end: 6 },).unwrap().max, 5.0,);

        assert_eq!(
            tree.query_dfs(0, Span { start: 1, end: 3 },).unwrap().max,
            2.0
        );

        assert_eq!(
            tree.query_dfs(0, Span { start: 1, end: 7 },).unwrap().max,
            5.0
        );
    }

    #[test]
    fn min() {
        let (data, mut tree) = tree_data();
        tree.build(&data, 0, 0, data.len() - 1);

        assert_eq!(
            tree.query_dfs(0, Span { start: 2, end: 6 },).unwrap().min,
            2.0,
        );

        assert_eq!(
            tree.query_dfs(0, Span { start: 1, end: 3 },).unwrap().min,
            1.0
        );

        assert_eq!(
            tree.query_dfs(0, Span { start: 1, end: 7 },).unwrap().min,
            1.0
        );
    }

    #[test]
    fn count() {
        let (data, mut tree) = tree_data();
        tree.build(&data, 0, 0, data.len() - 1);

        assert_eq!(
            tree.query_dfs(0, Span { start: 2, end: 6 },).unwrap().count,
            4,
        );

        assert_eq!(
            tree.query_dfs(0, Span { start: 4, end: 6 },).unwrap().count,
            2
        );

        assert_eq!(
            tree.query_dfs(0, Span { start: 1, end: 6 },).unwrap().count,
            5
        );

        assert_eq!(
            tree.query_dfs(0, Span { start: 0, end: 6 },).unwrap().count,
            6
        );
    }
}
