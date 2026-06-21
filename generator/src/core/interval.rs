#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interval {
    pub start: u32,
    pub end: u32,
}

impl Interval {
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    pub fn contains(&self, point: u32) -> bool {
        self.start <= point && point < self.end
    }

    pub fn merge(mut intervals: Vec<Interval>) -> Vec<Interval> {
        if intervals.is_empty() {
            return intervals;
        }
        intervals.sort_by_key(|i| i.start);
        let mut merged = Vec::new();
        let mut cur = intervals[0];
        for next in intervals.into_iter().skip(1) {
            if next.start <= cur.end {
                cur.end = cur.end.max(next.end);
            } else {
                merged.push(cur);
                cur = next;
            }
        }
        merged.push(cur);
        merged
    }

    pub fn complement(mut intervals: Vec<Interval>, max_codepoint: u32) -> Vec<Interval> {
        if intervals.is_empty() {
            return vec![Interval::new(0, max_codepoint + 1)];
        }
        intervals.sort_by_key(|i| i.start);
        let mut result = Vec::new();
        let mut cur = 0;
        for int in intervals {
            if int.start > cur {
                result.push(Interval::new(cur, int.start));
            }
            cur = cur.max(int.end);
        }
        if cur <= max_codepoint {
            result.push(Interval::new(cur, max_codepoint + 1));
        }
        result
    }

    pub fn collect_boundaries(intervals: &[Interval]) -> Vec<u32> {
        let mut points = std::collections::BTreeSet::new();
        points.insert(0);
        points.insert(0x110000);
        for int in intervals {
            points.insert(int.start);
            points.insert(int.end);
        }
        points.into_iter().collect()
    }

    pub fn build_intervals(boundaries: &[u32]) -> Vec<Interval> {
        boundaries
            .windows(2)
            .filter(|w| w[0] < w[1])
            .map(|w| Interval::new(w[0], w[1]))
            .collect()
    }
}
