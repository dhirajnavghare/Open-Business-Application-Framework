use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

struct BucketState {
    tokens: f64,
    last_refill: Instant,
}

#[derive(Debug, PartialEq)]
pub enum RateLimitResult {
    Allowed { remaining: f64 },
    Rejected { retry_after_secs: f64 },
}

pub const DEFAULT_MAX_BUCKETS: usize = 50_000;

/// High-performance thread-safe Token Bucket Rate Limiter
/// Engineered following the System Design Primer pattern for traffic shaping and DoS mitigation.
///
/// # Example
/// ```
/// use obaf_core::rate_limiter::{TokenBucketRateLimiter, RateLimitResult};
///
/// let limiter = TokenBucketRateLimiter::new(10.0, 2.0);
/// match limiter.check_and_consume("tenant_alpha", 1.0) {
///     RateLimitResult::Allowed { remaining } => assert!(remaining >= 8.9),
///     RateLimitResult::Rejected { .. } => panic!("Request should be allowed"),
/// }
/// ```
#[derive(Clone)]
pub struct TokenBucketRateLimiter {
    buckets: Arc<RwLock<HashMap<String, BucketState>>>,
    capacity: f64,
    refill_rate_per_sec: f64,
    max_buckets: usize,
}

impl TokenBucketRateLimiter {
    /// Create a rate limiter with maximum burst capacity and continuous refill rate per second.
    pub fn new(capacity: f64, refill_rate_per_sec: f64) -> Self {
        Self::with_max_buckets(capacity, refill_rate_per_sec, DEFAULT_MAX_BUCKETS)
    }

    /// Create a rate limiter with a custom maximum bucket capacity ceiling.
    pub fn with_max_buckets(capacity: f64, refill_rate_per_sec: f64, max_buckets: usize) -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
            capacity,
            refill_rate_per_sec,
            max_buckets,
        }
    }

    /// Check if the client or tenant identified by `key` has enough tokens for the request.
    /// If sufficient tokens exist, decrements the bucket and returns `RateLimitResult::Allowed`.
    /// Otherwise, calculates wait time and returns `RateLimitResult::Rejected`.
    pub fn check_and_consume(&self, key: &str, requested_tokens: f64) -> RateLimitResult {
        let now = Instant::now();
        let mut map = self.buckets.write().unwrap_or_else(|e| e.into_inner());

        // State Exhaustion Protection (DoS): enforce maximum bucket capacity ceiling
        if !map.contains_key(key) && map.len() >= self.max_buckets {
            // 1. Purge buckets idle for > 60 seconds
            map.retain(|_, state| now.duration_since(state.last_refill).as_secs_f64() < 60.0);

            // 2. If still at capacity, evict the least recently touched bucket
            if map.len() >= self.max_buckets {
                if let Some(oldest_key) = map
                    .iter()
                    .min_by_key(|(_, state)| state.last_refill)
                    .map(|(k, _)| k.clone())
                {
                    map.remove(&oldest_key);
                }
            }
        }

        let state = map.entry(key.to_string()).or_insert_with(|| BucketState {
            tokens: self.capacity,
            last_refill: now,
        });

        // Refill tokens based on elapsed duration
        let elapsed = now.duration_since(state.last_refill).as_secs_f64();
        state.tokens = (state.tokens + elapsed * self.refill_rate_per_sec).min(self.capacity);
        state.last_refill = now;

        if state.tokens >= requested_tokens {
            state.tokens -= requested_tokens;
            RateLimitResult::Allowed {
                remaining: state.tokens,
            }
        } else {
            let deficit = requested_tokens - state.tokens;
            let retry_after_secs = if self.refill_rate_per_sec > 0.0 {
                deficit / self.refill_rate_per_sec
            } else {
                f64::INFINITY
            };
            RateLimitResult::Rejected { retry_after_secs }
        }
    }

    /// Prune buckets that have not been touched for longer than max_idle_secs to prevent memory leaks.
    pub fn prune_idle(&self, max_idle_secs: f64) {
        let now = Instant::now();
        let mut map = self.buckets.write().unwrap_or_else(|e| e.into_inner());
        map.retain(|_, state| now.duration_since(state.last_refill).as_secs_f64() < max_idle_secs);
    }

    /// Return the current number of tracked buckets in memory.
    pub fn buckets_len(&self) -> usize {
        let map = self.buckets.read().unwrap_or_else(|e| e.into_inner());
        map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_token_bucket_burst_and_rejection() {
        // Capacity: 3 requests, refill: 1 token/sec
        let limiter = TokenBucketRateLimiter::new(3.0, 1.0);

        // 3 burst requests should pass
        assert!(matches!(
            limiter.check_and_consume("client_1", 1.0),
            RateLimitResult::Allowed { .. }
        ));
        assert!(matches!(
            limiter.check_and_consume("client_1", 1.0),
            RateLimitResult::Allowed { .. }
        ));
        assert!(matches!(
            limiter.check_and_consume("client_1", 1.0),
            RateLimitResult::Allowed { .. }
        ));

        // 4th request must be rejected
        match limiter.check_and_consume("client_1", 1.0) {
            RateLimitResult::Rejected { retry_after_secs } => {
                assert!(retry_after_secs > 0.0);
            }
            _ => panic!("Expected rate limit rejection"),
        }

        // Another client key has its own isolated quota
        assert!(matches!(
            limiter.check_and_consume("client_2", 1.0),
            RateLimitResult::Allowed { .. }
        ));
    }

    #[test]
    fn test_token_bucket_refill() {
        let limiter = TokenBucketRateLimiter::new(1.0, 20.0); // 20 tokens per second
        assert!(matches!(
            limiter.check_and_consume("fast_client", 1.0),
            RateLimitResult::Allowed { .. }
        ));

        // Immediately exhausted
        assert!(matches!(
            limiter.check_and_consume("fast_client", 1.0),
            RateLimitResult::Rejected { .. }
        ));

        // Sleep 60ms => should refill > 1 token
        sleep(Duration::from_millis(60));
        assert!(matches!(
            limiter.check_and_consume("fast_client", 1.0),
            RateLimitResult::Allowed { .. }
        ));
    }
}
