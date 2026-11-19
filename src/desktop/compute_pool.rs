use std::fmt::Write as _;
use std::thread;

use super::http_codec::escape_json;

/// Scored item returned by `NativeComputePool::fuzzy_filter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputeMatch {
    pub index: usize,
    pub score: u32,
    pub value: String,
}

/// Multithreaded native Rust compute pool for off-thread fuzzy search, sorting, and hashing.
#[derive(Debug, Clone)]
pub struct NativeComputePool {
    worker_threads: usize,
}

impl Default for NativeComputePool {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeComputePool {
    pub fn new() -> Self {
        let worker_threads = thread::available_parallelism()
            .map_or(2, usize::from)
            .clamp(1, 8);
        Self { worker_threads }
    }

    pub const fn with_threads(threads: usize) -> Self {
        let clamped = if threads == 0 {
            1
        } else if threads > 16 {
            16
        } else {
            threads
        };
        Self {
            worker_threads: clamped,
        }
    }

    pub const fn worker_threads(&self) -> usize {
        self.worker_threads
    }

    /// Scores a candidate string against a lowercase query needle.
    pub fn score_candidate(candidate: &str, needle_lower: &str) -> Option<u32> {
        if needle_lower.is_empty() {
            return Some(1);
        }
        let cand_lower = candidate.to_ascii_lowercase();
        if cand_lower == needle_lower {
            return Some(1000);
        }
        if cand_lower.starts_with(needle_lower) {
            let len_bonus = u32::try_from(candidate.len()).map_or(0, |l| 80_u32.saturating_sub(l));
            return Some(600 + len_bonus);
        }
        if let Some(pos) = cand_lower.find(needle_lower) {
            let pos_bonus = u32::try_from(pos).map_or(0, |p| 60_u32.saturating_sub(p * 2));
            return Some(350 + pos_bonus);
        }

        // Ordered subsequence match
        let mut needle_chars = needle_lower.chars().peekable();
        let mut consecutive_bonus = 0_u32;
        let mut prev_matched = false;
        for ch in cand_lower.chars() {
            if needle_chars.peek() == Some(&ch) {
                let _ = needle_chars.next();
                if prev_matched {
                    consecutive_bonus = consecutive_bonus.saturating_add(15);
                }
                prev_matched = true;
                if needle_chars.peek().is_none() {
                    break;
                }
            } else {
                prev_matched = false;
            }
        }

        if needle_chars.peek().is_none() {
            Some(100 + consecutive_bonus)
        } else {
            None
        }
    }

    fn score_slice(slice: &[String], offset: usize, needle_lower: &str) -> Vec<ComputeMatch> {
        let mut out = Vec::new();
        for (idx, item) in slice.iter().enumerate() {
            if let Some(score) = Self::score_candidate(item, needle_lower) {
                out.push(ComputeMatch {
                    index: offset + idx,
                    score,
                    value: item.clone(),
                });
            }
        }
        out
    }

    /// Executes fuzzy filtering across `items`, parallelizing across OS threads when `items.len() >= 64`.
    pub fn fuzzy_filter(&self, items: &[String], query: &str, limit: usize) -> Vec<ComputeMatch> {
        if items.is_empty() || limit == 0 {
            return Vec::new();
        }
        let needle_lower = query.trim().to_ascii_lowercase();
        let mut matches = if items.len() >= 64 && self.worker_threads > 1 {
            let chunk_size = items.len().div_ceil(self.worker_threads).max(1);
            thread::scope(|scope| {
                let mut handles = Vec::new();
                for (chunk_idx, chunk) in items.chunks(chunk_size).enumerate() {
                    let offset = chunk_idx * chunk_size;
                    let needle_ref = needle_lower.as_str();
                    handles.push(scope.spawn(move || Self::score_slice(chunk, offset, needle_ref)));
                }
                let mut combined = Vec::new();
                for handle in handles {
                    if let Ok(mut part) = handle.join() {
                        combined.append(&mut part);
                    }
                }
                combined
            })
        } else {
            Self::score_slice(items, 0, &needle_lower)
        };

        matches.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.index.cmp(&b.index)));
        if matches.len() > limit {
            matches.truncate(limit);
        }
        matches
    }

    /// Serializes `fuzzy_filter` results as a JSON array for the JS bridge.
    pub fn fuzzy_filter_json(&self, items: &[String], query: &str, limit: usize) -> String {
        let matches = self.fuzzy_filter(items, query, limit);
        let mut json = String::from("[");
        for (i, m) in matches.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            let _ = write!(
                json,
                r#"{{"index":{},"score":{},"value":"{}"}}"#,
                m.index,
                m.score,
                escape_json(&m.value)
            );
        }
        json.push(']');
        json
    }

    /// Computes a deterministic 64-bit FNV-1a digest formatted as 16 lowercase hex characters.
    pub fn fnv1a_hex(&self, input: &str) -> String {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in input.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        format!("{hash:016x}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_filters_items_across_single_and_multithreaded_batches() {
        let pool = NativeComputePool::with_threads(4);
        assert_eq!(pool.worker_threads(), 4);

        let mut catalog: Vec<String> = (1..=120).map(|i| format!("pokemon-entry-{i:03}")).collect();
        catalog.push(String::from("pikachu"));
        catalog.push(String::from("pichu"));
        catalog.push(String::from("raichu"));

        let top = pool.fuzzy_filter(&catalog, "pikachu", 5);
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].value, "pikachu");
        assert_eq!(top[0].score, 1000);

        let sub = pool.fuzzy_filter(&catalog, "chu", 5);
        assert_eq!(sub.len(), 3);

        let json = pool.fuzzy_filter_json(&catalog, "pikachu", 1);
        assert!(json.contains(r#""value":"pikachu""#));

        let digest_a = pool.fnv1a_hex("ng-native-engine");
        let digest_b = pool.fnv1a_hex("ng-native-engine");
        assert_eq!(digest_a.len(), 16);
        assert_eq!(digest_a, digest_b);
    }
}
