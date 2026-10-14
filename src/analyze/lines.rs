//! Line positions and code-line ownership.

/// Maps byte offsets to 1-based line numbers.
pub struct LineIndex {
    starts: Vec<u32>,
}

impl LineIndex {
    /// Builds the index for normalized (LF-only) source text.
    pub fn new(source: &str) -> Self {
        let mut starts = vec![0];
        for (i, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                starts.push(offset(i + 1));
            }
        }
        LineIndex { starts }
    }

    /// 1-based line containing `pos`.
    pub fn line(&self, pos: u32) -> u32 {
        let idx = self.starts.partition_point(|&start| start <= pos);
        offset(idx.max(1))
    }
}

fn offset(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// Offset of the first code character on every line that has code.
///
/// Whitespace and comments are skipped; `comments` holds `(start, end)` byte
/// ranges sorted by start, delimiters included.
pub fn first_code_offsets(source: &str, comments: &[(u32, u32)]) -> Vec<u32> {
    let bytes = source.as_bytes();
    let mut result = Vec::new();
    let mut comment = 0;
    let mut i = 0usize;
    let mut line_done = false;
    while i < bytes.len() {
        if let Some(&(start, end)) = comments.get(comment)
            && offset(i) >= start
        {
            let end = end as usize;
            if bytes[i.min(end)..end.min(bytes.len())].contains(&b'\n') {
                line_done = false;
            }
            i = end.max(i + 1);
            comment += 1;
            continue;
        }
        match bytes[i] {
            b'\n' => line_done = false,
            b' ' | b'\t' | b'\r' | 0x0b | 0x0c => {}
            _ if !line_done => {
                result.push(offset(i));
                line_done = true;
            }
            _ => {}
        }
        i += 1;
    }
    result
}

/// Counts, for every function span, the code lines it owns.
///
/// A line belongs to the innermost span containing its first code character.
/// Spans must be properly nested; `spans[i]` is `(start, end)`.
pub fn owned_line_counts(line_offsets: &[u32], spans: &[(u32, u32)]) -> Vec<u32> {
    let mut order: Vec<usize> = (0..spans.len()).collect();
    order.sort_by(|&a, &b| {
        let (sa, ea) = spans[a];
        let (sb, eb) = spans[b];
        sa.cmp(&sb).then(eb.cmp(&ea)).then(a.cmp(&b))
    });
    let mut counts = vec![0u32; spans.len()];
    let mut stack: Vec<usize> = Vec::new();
    let mut next = 0;
    for &pos in line_offsets {
        while next < order.len() && spans[order[next]].0 <= pos {
            let start = spans[order[next]].0;
            while stack.last().is_some_and(|&top| spans[top].1 <= start) {
                stack.pop();
            }
            stack.push(order[next]);
            next += 1;
        }
        while stack.last().is_some_and(|&top| spans[top].1 <= pos) {
            stack.pop();
        }
        if let Some(&owner) = stack.last() {
            counts[owner] += 1;
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_index_should_map_offsets_to_lines() {
        let index = LineIndex::new("a\nbc\n\nd");
        assert_eq!(
            [0, 1, 2, 4, 5, 6].map(|p| index.line(p)),
            [1, 1, 2, 2, 3, 4]
        );
    }

    #[test]
    fn first_code_offsets_should_skip_blank_and_comment_lines() {
        let src = "a\n  // c\n\n/* x\n y */ b\nc /* z */\n";
        let comments = [(4, 8), (10, 20), (25, 32)];
        assert_eq!(first_code_offsets(src, &comments), vec![0, 21, 23]);
    }

    #[test]
    fn owned_line_counts_should_give_lines_to_innermost_span() {
        let counts = owned_line_counts(&[0, 10, 20, 30], &[(0, 40), (5, 25), (20, 21)]);
        assert_eq!(counts, vec![2, 1, 1]);
    }
}
