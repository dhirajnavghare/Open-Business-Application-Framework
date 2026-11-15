use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

struct BreakerInternal {
    state: CircuitState,
    consecutive_failures: u32,
    consecutive_successes: u32,
    last_failure_time: Option<Instant>,
}

/// Circuit Breaker Pattern (System Design Primer)
/// Prevents cascading failures and resource exhaustion by failing fast when downstream services are unstable.
///
/// # Example
/// ```
/// use obaf_core::circuit_breaker::{CircuitBreaker, CircuitState};
/// use std::time::Duration;
///
/// let breaker = CircuitBreaker::new(3, Duration::from_secs(5), 2);
/// assert_eq!(breaker.state(), CircuitState::Closed);
/// let res = breaker.call(|| Ok::<_, &'static str>(42));
/// assert_eq!(res, Ok(42));
/// ```
#[derive(Clone)]
pub struct CircuitBreaker {
    internal: Arc<RwLock<BreakerInternal>>,
    failure_threshold: u32,
    recovery_timeout: Duration,
    half_open_success_threshold: u32,
}

#[derive(Debug, PartialEq)]
pub enum CircuitBreakerError<E> {
    CircuitOpen,
    ExecutionFailed(E),
}

impl CircuitBreaker {
    /// Create a new Circuit Breaker with specified thresholds.
    pub fn new(
        failure_threshold: u32,
        recovery_timeout: Duration,
        half_open_success_threshold: u32,
    ) -> Self {
        Self {
            internal: Arc::new(RwLock::new(BreakerInternal {
                state: CircuitState::Closed,
                consecutive_failures: 0,
                consecutive_successes: 0,
                last_failure_time: None,
            })),
            failure_threshold,
            recovery_timeout,
            half_open_success_threshold,
        }
    }

    /// Check if the circuit breaker permits an operation to proceed.
    pub fn can_execute(&self) -> bool {
        let mut lock = self.internal.write().unwrap_or_else(|e| e.into_inner());
        match lock.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_fail) = lock.last_failure_time {
                    if Instant::now().duration_since(last_fail) >= self.recovery_timeout {
                        // Recovery timeout expired, transition to HalfOpen for probe
                        lock.state = CircuitState::HalfOpen;
                        lock.consecutive_successes = 0;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a successful execution, updating the state machine.
    pub fn record_success(&self) {
        let mut lock = self.internal.write().unwrap_or_else(|e| e.into_inner());
        match lock.state {
            CircuitState::Closed => {
                lock.consecutive_failures = 0;
            }
            CircuitState::HalfOpen => {
                lock.consecutive_successes += 1;
                if lock.consecutive_successes >= self.half_open_success_threshold {
                    // Restored downstream stability: reset to Closed
                    lock.state = CircuitState::Closed;
                    lock.consecutive_failures = 0;
                    lock.consecutive_successes = 0;
                    lock.last_failure_time = None;
                }
            }
            CircuitState::Open => {}
        }
    }

    /// Record a failure, potentially tripping the circuit to Open.
    pub fn record_failure(&self) {
        let mut lock = self.internal.write().unwrap_or_else(|e| e.into_inner());
        lock.consecutive_failures += 1;
        lock.last_failure_time = Some(Instant::now());

        match lock.state {
            CircuitState::Closed => {
                if lock.consecutive_failures >= self.failure_threshold {
                    lock.state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in HalfOpen immediately trips back to Open
                lock.state = CircuitState::Open;
            }
            CircuitState::Open => {}
        }
    }

    /// Get current state of the circuit breaker.
    pub fn state(&self) -> CircuitState {
        let lock = self.internal.read().unwrap_or_else(|e| e.into_inner());
        lock.state
    }

    /// Wrap and execute an operation protected by this circuit breaker.
    pub fn call<F, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Result<T, E>,
    {
        if !self.can_execute() {
            return Err(CircuitBreakerError::CircuitOpen);
        }

        match f() {
            Ok(val) => {
                self.record_success();
                Ok(val)
            }
            Err(err) => {
                self.record_failure();
                Err(CircuitBreakerError::ExecutionFailed(err))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_circuit_breaker_transitions() {
        let breaker = CircuitBreaker::new(2, Duration::from_millis(50), 2);
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(breaker.can_execute());

        // 1st failure
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Closed);

        // 2nd failure trips circuit to Open
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        assert!(!breaker.can_execute());

        // Fast fail via call()
        let result: Result<(), CircuitBreakerError<&str>> = breaker.call(|| Ok(()));
        assert_eq!(result, Err(CircuitBreakerError::CircuitOpen));

        // Sleep to exceed recovery timeout
        sleep(Duration::from_millis(60));

        // can_execute triggers HalfOpen transition
        assert!(breaker.can_execute());
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        // 1st success in HalfOpen
        breaker.record_success();
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        // 2nd success resets to Closed
        breaker.record_success();
        assert_eq!(breaker.state(), CircuitState::Closed);
    }
}
