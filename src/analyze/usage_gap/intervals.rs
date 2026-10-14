//! Counts wholly intervening statements with an offline sweep and prefix sums.

use oxc_span::Span;

struct PrefixCounts(Vec<u32>);

impl PrefixCounts {
    fn add(&mut self, rank: usize) {
        let mut i = rank + 1;
        while i < self.0.len() {
            self.0[i] += 1;
            i += i.isolate_lowest_one();
        }
    }

    fn prefix(&self, mut end: usize) -> u32 {
        let mut sum = 0;
        while end > 0 {
            sum += self.0[end];
            end &= end - 1;
        }
        sum
    }
}

pub(super) fn count(statements: &mut [Span], ranges: &[(u32, u32)]) -> Vec<u32> {
    let mut starts: Vec<_> = statements.iter().map(|s| s.start).collect();
    starts.sort_unstable();
    statements.sort_unstable_by_key(|s| s.end);
    let mut order: Vec<_> = (0..ranges.len()).collect();
    order.sort_unstable_by_key(|&i| ranges[i].1);
    let mut tree = PrefixCounts(vec![0; starts.len() + 1]);
    let mut next = 0;
    let mut counts = vec![0; ranges.len()];
    for i in order {
        let (left, right) = ranges[i];
        while next < statements.len() && statements[next].end <= right {
            tree.add(starts.partition_point(|&start| start < statements[next].start));
            next += 1;
        }
        let before = starts.partition_point(|&start| start < left);
        counts[i] = tree.prefix(starts.len()) - tree.prefix(before);
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sweep_should_match_exhaustive_containment_counts() {
        let statements: Vec<_> = (0..50)
            .flat_map(|start| {
                (start + 1..=50)
                    .step_by(7)
                    .map(move |end| Span::new(start, end))
            })
            .collect();
        let ranges: Vec<_> = (0..=50)
            .flat_map(|left| (left..=50).map(move |right| (left, right)))
            .collect();
        let expected: Vec<_> = ranges
            .iter()
            .map(|&(left, right)| {
                statements
                    .iter()
                    .filter(|s| s.start >= left && s.end <= right)
                    .count() as u32
            })
            .collect();
        assert_eq!(count(&mut statements.clone(), &ranges), expected);
    }
}
