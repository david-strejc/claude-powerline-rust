# Performance Improvement Plan for Claude Powerline

## Executive Summary

After analysis by O3 and Gemini 2.5 Pro models, we've identified opportunities for **4-6x performance improvements** with targeted optimizations. The primary bottleneck is redundant multi-pass data processing, followed by JSON parsing overhead and excessive string allocations.

## Performance Bottlenecks (Ordered by Impact)

### 1. 🔴 **CRITICAL: Redundant Multi-Pass Processing**
**Current Issue:**
- Functions `calculate_total_cost`, `calculate_token_breakdown`, and `calculate_weighted_tokens` each independently:
  - Group all entries by session
  - Sort each session by timestamp
  - Calculate deltas from cumulative values
- This results in 3x redundant processing of the same data

**Impact:** 60-70% of total CPU time wasted on repeated operations

**Solution:**
```rust
pub struct ProcessedMetrics {
    pub total_cost: f64,
    pub token_breakdown: TokenBreakdown,
    pub weighted_tokens: u32,
}

impl PricingService {
    pub fn process_all_metrics(&self, entries: Vec<ParsedEntry>) -> ProcessedMetrics {
        // Group and sort ONCE
        let grouped = self.group_and_sort_by_session(entries);
        
        // Process sessions in parallel, calculating all metrics in one pass
        let results: Vec<SessionMetrics> = grouped.par_iter()
            .map(|(_, session)| self.calculate_session_metrics(session))
            .collect();
        
        // Aggregate results
        ProcessedMetrics {
            total_cost: results.iter().map(|r| r.cost).sum(),
            token_breakdown: aggregate_tokens(&results),
            weighted_tokens: results.iter().map(|r| r.weighted).sum(),
        }
    }
}
```

**Expected Improvement:** 2-3x speedup
**Effort:** 2-3 hours
**Priority:** P0 - Must implement

### 2. 🔴 **HIGH: JSON Parsing Performance**
**Current Issue:**
- Using `serde_json::from_str` for each line (80-90% of parsing time)
- Double conversion: JSON → Value → HashMap
- String allocations for each line read

**Impact:** 30-40% of total runtime for large transcript files

**Solution:**
```rust
// Add to Cargo.toml
simd-json = { version = "0.13", features = ["serde", "allow-non-simd"] }

// Use SIMD-accelerated parsing
use simd_json::BorrowedValue;

fn parse_jsonl_line(&self, line: &str) -> Result<Option<ParsedEntry>> {
    let mut bytes = line.as_bytes().to_vec();
    let raw: BorrowedValue = simd_json::to_borrowed_value(&mut bytes)?;
    // Direct field access without intermediate conversion
}
```

**Expected Improvement:** 3-4x faster JSON parsing
**Effort:** 1 hour
**Priority:** P0 - High impact, low effort

### 3. 🟡 **MEDIUM: Excessive String Allocations**
**Current Issues:**
- Cloning `source_file` string for every entry (10,000+ clones per file)
- Creating formatted strings for deduplication keys
- String allocations for HashMap keys

**Impact:** 50% more memory usage than necessary, GC pressure

**Solutions:**

#### A. String Interning
```rust
// Add to Cargo.toml
lasso = "0.7"

use lasso::{Rodeo, Spur};

struct InternedStrings {
    rodeo: Rodeo,
    source_files: HashMap<PathBuf, Spur>,
}
```

#### B. Arc for Shared Strings
```rust
// Instead of: entry.source_file = Some(source_file.clone());
entry.source_file = Some(Arc::new(source_file));
```

#### C. Hash-based Deduplication
```rust
use ahash::AHasher;

// Instead of: format!("{}:{}", message_id, request_id)
let mut hasher = AHasher::default();
hasher.write(message_id.as_bytes());
hasher.write(request_id.as_bytes());
let hash_key = hasher.finish();
```

**Expected Improvement:** 20-50% memory reduction, 15% speed improvement
**Effort:** 1-2 hours
**Priority:** P1 - Significant gains

### 4. 🟢 **EASY: Faster HashMap Implementation**
**Current Issue:** Using std HashMap with cryptographically secure (slow) hasher

**Solution:**
```rust
// Add to Cargo.toml
ahash = "0.8"
# OR
rustc-hash = "1.1"

// Replace all uses
use ahash::AHashMap as HashMap;
use ahash::AHashSet as HashSet;
```

**Expected Improvement:** 10-20% speedup on all hash operations
**Effort:** 5 minutes
**Priority:** P0 - Quick win

### 5. 🟢 **EASY: Parallelize More Operations**
**Current Issues:**
- Serial sorting of large entry vectors
- Serial processing of sessions after grouping

**Solutions:**
```rust
// Parallel sort
entries.par_sort_by_key(|e| e.timestamp);

// Parallel session processing
let results: Vec<_> = sessions.par_iter()
    .map(|(_key, session)| process_session(session))
    .collect();
```

**Expected Improvement:** 20-40% speedup on multi-core systems
**Effort:** 30 minutes
**Priority:** P1 - Easy parallelization

## Quick Wins Checklist

- [ ] Replace HashMap with `ahash::AHashMap` (5 min)
- [ ] Use `par_sort_by_key` for sorting (5 min)
- [ ] Pre-allocate vectors with `with_capacity()` (15 min)
- [ ] Reuse string buffers in loops (15 min)
- [ ] Add `#[inline]` to hot path functions (10 min)
- [ ] Replace `walkdir` with `ignore` crate (15 min)

## Implementation Roadmap

### Phase 1: Quick Wins (Week 1)
1. ✅ Switch to faster HashMap (ahash)
2. ✅ Enable parallel sorting
3. ✅ Pre-allocate collections
4. ✅ Add inline hints

**Expected Gain:** 25-30% improvement

### Phase 2: Core Refactoring (Week 2)
1. ✅ Implement single-pass processing pipeline
2. ✅ Consolidate grouping/sorting logic
3. ✅ Parallelize session processing

**Expected Gain:** 2-3x improvement

### Phase 3: Advanced Optimizations (Week 3)
1. ✅ Integrate SIMD JSON parsing
2. ✅ Implement string interning
3. ✅ Optimize deduplication with hashing

**Expected Gain:** Additional 2x improvement

### Phase 4: Architecture Improvements (Future)
1. ⏳ Add caching layer for repeated renders
2. ⏳ Consider daemon architecture for instant queries
3. ⏳ Implement incremental processing for new entries

## Performance Metrics

### Current Baseline (v1.1.0)
- 10K entries: ~500ms
- 100K entries: ~5s
- Memory usage: ~200MB for 100K entries

### Target Performance (After Optimizations)
- 10K entries: <100ms
- 100K entries: <1s
- Memory usage: <100MB for 100K entries

## Testing Strategy

1. Create benchmark suite with varying dataset sizes
2. Profile with `cargo flamegraph` before/after each optimization
3. Monitor memory usage with `valgrind --tool=massif`
4. Test on different hardware (2-core, 8-core, 16-core)

## Risk Mitigation

- Keep original implementation as fallback
- Add feature flags for experimental optimizations
- Thoroughly test cumulative token calculation logic
- Ensure backward compatibility with existing transcript formats

## Dependencies to Add

```toml
[dependencies]
ahash = "0.8"           # Faster hashing
simd-json = "0.13"      # SIMD JSON parsing
lasso = "0.7"           # String interning
ignore = "0.5"          # Faster directory walking

[dev-dependencies]
criterion = "0.5"       # Benchmarking
flamegraph = "0.6"      # Performance profiling
```

## Success Criteria

- [ ] 4x performance improvement on typical workloads
- [ ] 50% memory usage reduction
- [ ] Sub-100ms response time for prompt rendering
- [ ] No regression in accuracy or functionality
- [ ] Maintainable, well-documented code

## References

- O3 Analysis: Focus on SIMD JSON, string interning, unified processing
- Gemini 2.5 Pro Analysis: Emphasis on parallelization, caching, architectural improvements
- Rust Performance Book: https://nnethercote.github.io/perf-book/
- Flame Graph Profiling: https://github.com/flamegraph-rs/flamegraph

---

*Last Updated: January 2025*
*Analysis performed by: O3 and Gemini 2.5 Pro models*
*Target Release: v1.2.0*