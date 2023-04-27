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
    pub start: u64,
    pub end: u64,
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
    pub fn new(span: Span) -> Self {
        // find the smallest power of 2 that is greater than or equal to the range of the span
        // and then multiply it by 2 to account for the internal nodes in a complete binary tree.
        let tree_size = 2 * (2usize.pow(((span.end - span.start) as f64).log2().ceil() as u32)) - 1;
        Self {
            tree: vec![ISegment::default(); tree_size],
        }
    }

    pub fn build(&mut self, values: Vec<ISegment>, index: usize, left: usize, right: usize) {
        if left == right {
            self.tree[index] = values[left].clone();
        } else {
            let mid: usize = left + (right - left) / 2;
            self.build(values.clone(), index * 2 + 1, left, mid);
            self.build(values.clone(), index * 2 + 2, mid + 1, right);
            self.tree[index] = ISegment {
                span: Span {
                    start: self.tree[index * 2 + 1].span.start,
                    end: self.tree[index * 2 + 2].span.end,
                },
                count: self.tree[index * 2 + 1].count + self.tree[index * 2 + 2].count,
                max: self.tree[index * 2 + 1]
                    .max
                    .max(self.tree[index * 2 + 2].max),
                min: self.tree[index * 2 + 1]
                    .min
                    .min(self.tree[index * 2 + 2].min),
                sum: self.tree[index * 2 + 1].sum + self.tree[index * 2 + 2].sum,
            };
        }
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

#[cfg(test)]
mod tests {
    use super::{ISegment, ISegmentIndex, Span};

    fn tree_data() -> (Vec<ISegment>, ISegmentIndex) {
        let mut data = vec![ISegment::default(); 6];
        for i in 0..data.len() {
            let time: u64 = i as u64;
            let val: f64 = i as f64 * 2.0;
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

        (
            data.clone(),
            ISegmentIndex::new(Span {
                start: data[0].span.start,
                end: data[data.len() - 1].span.end,
            }),
        )
    }

    #[test]
    fn build() {
        let (data, mut tree) = tree_data();
        tree.build(data.clone(), 0, 0, data.len() - 1);
        for i in 0..tree.tree.len() {
            println!("{:?}", tree.tree[i]);
        }

        assert_eq!(tree.tree[0].count, 6);
    }

    #[test]
    fn sum() {
        let (data, mut tree) = tree_data();
        tree.build(data.clone(), 0, 0, data.len() - 1);

        for i in 0..data.len() {
            print!("{:?} ", data[i].sum);
        }
        for i in 0..tree.tree.len() {
            println!("{:?}", tree.tree[i]);
        }

        assert_eq!(
            tree.query_bfs(Span { start: 1, end: 6 },).unwrap().sum,
            30.0,
        );

        assert_eq!(
            tree.query_bfs(Span { start: 1, end: 6 },).unwrap().sum,
            30.0,
        );

        assert_eq!(tree.query_bfs(Span { start: 1, end: 3 },).unwrap().sum, 6.0);

        assert_eq!(
            tree.query_bfs(Span { start: 0, end: 6 },).unwrap().sum,
            30.0
        );
    }

    #[test]
    fn max() {
        let (data, mut tree) = tree_data();
        tree.build(data.clone(), 0, 0, data.len() - 1);

        assert_eq!(
            tree.query_bfs(Span { start: 2, end: 6 },).unwrap().max,
            10.0,
        );

        assert_eq!(
            tree.query_dfs(0, Span { start: 1, end: 3 },).unwrap().max,
            4.0
        );

        assert_eq!(
            tree.query_dfs(0, Span { start: 1, end: 7 },).unwrap().max,
            10.0
        );
    }

    #[test]
    fn min() {
        let (data, mut tree) = tree_data();
        tree.build(data.clone(), 0, 0, data.len() - 1);

        assert_eq!(
            tree.query_dfs(0, Span { start: 2, end: 6 },).unwrap().min,
            4.0,
        );

        assert_eq!(
            tree.query_dfs(0, Span { start: 1, end: 3 },).unwrap().min,
            2.0
        );

        assert_eq!(
            tree.query_dfs(0, Span { start: 1, end: 7 },).unwrap().min,
            2.0
        );
    }

    #[test]
    fn count() {
        let (data, mut tree) = tree_data();
        tree.build(data.clone(), 0, 0, data.len() - 1);

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
