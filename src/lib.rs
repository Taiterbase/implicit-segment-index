// https://en.algorithmica.org/hpc/data-structures/binary-search#eytzinger-layout
// https://github.com/cockroachdb/pebble
// Few assumptions:
// - data provided to the index is in time ascending order.
// - data is immutable.
// - data is not sparse.
#[derive(Clone, Debug, Copy)]
// span is a half-open interval [start, end)
pub struct Span {
    pub start: u64,
    pub end: u64,
}

impl Default for Span {
    fn default() -> Self {
        Span { start: 0, end: 0 }
    }
}

#[derive(Clone, Debug, Copy)]
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

    pub fn query_sum(&self, index: usize, segment_span: Span, query_span: Span) -> f64 {
        if query_span.end < segment_span.start || segment_span.end < query_span.start {
            // no overlap
            return 0.0;
        }

        if query_span.start <= segment_span.start && segment_span.end <= query_span.end {
            // total overlap
            return self.tree[index].sum;
        }

        let mid: u64 = segment_span.start + (segment_span.end - segment_span.start) / 2;
        let left_res = self.query_sum(
            index * 2 + 1,
            Span {
                start: segment_span.start,
                end: mid,
            },
            query_span,
        );
        let right_res = self.query_sum(
            index * 2 + 2,
            Span {
                start: mid + 1,
                end: segment_span.end,
            },
            query_span,
        );

        return left_res + right_res;
    }
}

#[cfg(test)]
mod tests {
    use super::{ISegment, ISegmentIndex, Span};
    #[test]
    fn build() {
        let mut data = vec![ISegment::default(); 6];
        for i in 0..data.len() {
            let time: u64 = i as u64 + 1;
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
        let mut tree = ISegmentIndex::new(Span {
            start: data[0].span.start,
            end: data[data.len() - 1].span.end,
        });
        tree.build(data.clone(), 0, 0, data.len() - 1);
        for i in 0..tree.tree.len() {
            println!("{:?}", tree.tree[i]);
        }

        assert_eq!(tree.tree[0].count, 6);

        assert_eq!(
            tree.query_sum(0, Span { start: 0, end: 7 }, Span { start: 2, end: 6 }),
            18.0
        );

        assert_eq!(
            tree.query_sum(0, Span { start: 0, end: 7 }, Span { start: 1, end: 3 }),
            6.0
        );

        assert_eq!(
            tree.query_sum(0, Span { start: 0, end: 7 }, Span { start: 1, end: 7 }),
            30.0
        );
    }
}
