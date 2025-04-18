//! Service health checker with liveness and readiness probes.
//!
//! # Example
//!
//! ```rust
//! use philiprehberger_service_health::HealthChecker;
//!
//! let mut checker = HealthChecker::new();
//! checker.add_liveness_check("ping", || Ok(()));
//! let report = checker.check_liveness();
//! assert!(report.is_healthy());
//! ```

use serde::{Serialize, Serializer};
use std::fmt;
use std::time::{Duration, Instant, SystemTime};

/// Health status of a service check.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Healthy => write!(f, "healthy"),
            Status::Degraded(msg) => write!(f, "degraded: {msg}"),
            Status::Unhealthy(msg) => write!(f, "unhealthy: {msg}"),
        }
    }
}

impl Serialize for Status {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

fn serialize_duration_ms<S: Serializer>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_f64(duration.as_secs_f64() * 1000.0)
}

/// Result of a single health check.
#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub name: String,
    pub status: Status,
    #[serde(rename = "duration_ms", serialize_with = "serialize_duration_ms")]
    pub duration: Duration,
}

/// Aggregated health report containing all check results.
#[derive(Debug, Clone, Serialize)]
pub struct HealthReport {
    pub status: Status,
    pub checks: Vec<CheckResult>,
    pub timestamp: u64,
}

impl HealthReport {
    /// Serialize the report to a pretty-printed JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }

    /// Returns true if the overall status is Healthy.
    pub fn is_healthy(&self) -> bool {
        self.status == Status::Healthy
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CheckKind {
    Liveness,
    Readiness,
    Both,
}

type CheckFn = Box<dyn Fn() -> Status + Send + Sync>;

/// Registry for health checks with liveness and readiness probe support.
pub struct HealthChecker {
    checks: Vec<(CheckKind, String, CheckFn)>,
}

impl fmt::Debug for HealthChecker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HealthChecker")
            .field("check_count", &self.checks.len())
            .finish()
    }
}

impl HealthChecker {
    /// Create a new empty health checker.
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    /// Add a general health check that runs for both liveness and readiness probes.
    pub fn add_check(
        &mut self,
        name: impl Into<String>,
        check: impl Fn() -> Status + Send + Sync + 'static,
    ) {
        self.checks.push((CheckKind::Both, name.into(), Box::new(check)));
    }

    /// Add a liveness-only health check.
    pub fn add_liveness_check(
        &mut self,
        name: impl Into<String>,
        check: impl Fn() -> Status + Send + Sync + 'static,
    ) {
        self.checks.push((CheckKind::Liveness, name.into(), Box::new(check)));
    }

    /// Add a readiness-only health check.
    pub fn add_readiness_check(
        &mut self,
        name: impl Into<String>,
        check: impl Fn() -> Status + Send + Sync + 'static,
    ) {
        self.checks.push((CheckKind::Readiness, name.into(), Box::new(check)));
    }

    /// Run all registered health checks and return a report.
    pub fn check_health(&self) -> HealthReport {
        self.run_checks(|_| true)
    }

    /// Run only liveness checks (Liveness + Both) and return a report.
    pub fn check_liveness(&self) -> HealthReport {
        self.run_checks(|kind| kind == CheckKind::Liveness || kind == CheckKind::Both)
    }

    /// Run only readiness checks (Readiness + Both) and return a report.
    pub fn check_readiness(&self) -> HealthReport {
        self.run_checks(|kind| kind == CheckKind::Readiness || kind == CheckKind::Both)
    }

    fn run_checks(&self, filter: impl Fn(CheckKind) -> bool) -> HealthReport {
        let mut results = Vec::new();

        for (kind, name, check) in &self.checks {
            if !filter(*kind) {
                continue;
            }
            let start = Instant::now();
            let status = check();
            let duration = start.elapsed();
            results.push(CheckResult {
                name: name.clone(),
                status,
                duration,
            });
        }

        let overall = compute_overall_status(&results);
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        HealthReport {
            status: overall,
            checks: results,
            timestamp,
        }
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

fn compute_overall_status(results: &[CheckResult]) -> Status {
    let mut has_degraded = false;
    for result in results {
        match &result.status {
            Status::Unhealthy(msg) => return Status::Unhealthy(msg.clone()),
            Status::Degraded(_) => has_degraded = true,
            Status::Healthy => {}
        }
    }
    if has_degraded {
        // Find the first degraded message for the overall status
        for result in results {
            if let Status::Degraded(msg) = &result.status {
                return Status::Degraded(msg.clone());
            }
        }
    }
    Status::Healthy
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn healthy_check() {
        let mut checker = HealthChecker::new();
        checker.add_check("test", || Status::Healthy);
        let report = checker.check_health();
        assert_eq!(report.status, Status::Healthy);
        assert_eq!(report.checks.len(), 1);
    }

    #[test]
    fn degraded_check() {
        let mut checker = HealthChecker::new();
        checker.add_check("test", || Status::Degraded("slow".into()));
        let report = checker.check_health();
        assert_eq!(report.status, Status::Degraded("slow".into()));
    }

    #[test]
    fn unhealthy_check() {
        let mut checker = HealthChecker::new();
        checker.add_check("test", || Status::Unhealthy("down".into()));
        let report = checker.check_health();
        assert_eq!(report.status, Status::Unhealthy("down".into()));
    }

    #[test]
    fn multiple_checks_overall() {
        let mut checker = HealthChecker::new();
        checker.add_check("ok", || Status::Healthy);
        checker.add_check("slow", || Status::Degraded("lag".into()));
        checker.add_check("broken", || Status::Unhealthy("crash".into()));
        let report = checker.check_health();
        assert_eq!(report.status, Status::Unhealthy("crash".into()));
        assert_eq!(report.checks.len(), 3);
    }

    #[test]
    fn liveness_vs_readiness() {
        let mut checker = HealthChecker::new();
        checker.add_liveness_check("live", || Status::Healthy);
        checker.add_readiness_check("ready", || Status::Healthy);

        let liveness = checker.check_liveness();
        assert_eq!(liveness.checks.len(), 1);
        assert_eq!(liveness.checks[0].name, "live");

        let readiness = checker.check_readiness();
        assert_eq!(readiness.checks.len(), 1);
        assert_eq!(readiness.checks[0].name, "ready");
    }

    #[test]
    fn check_health_runs_all() {
        let mut checker = HealthChecker::new();
        checker.add_liveness_check("live", || Status::Healthy);
        checker.add_readiness_check("ready", || Status::Healthy);
        checker.add_check("general", || Status::Healthy);

        let report = checker.check_health();
        assert_eq!(report.checks.len(), 3);
    }

    #[test]
    fn is_healthy() {
        let mut checker = HealthChecker::new();
        checker.add_check("ok", || Status::Healthy);
        let report = checker.check_health();
        assert!(report.is_healthy());

        let mut checker2 = HealthChecker::new();
        checker2.add_check("slow", || Status::Degraded("lag".into()));
        let report2 = checker2.check_health();
        assert!(!report2.is_healthy());
    }

    #[test]
    fn to_json() {
        let mut checker = HealthChecker::new();
        checker.add_check("db", || Status::Healthy);
        let report = checker.check_health();
        let json = report.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["status"], "healthy");
        assert!(parsed["checks"].is_array());
        assert_eq!(parsed["checks"][0]["name"], "db");
        assert!(parsed["timestamp"].is_u64());
        assert!(parsed["checks"][0]["duration_ms"].is_f64());
    }

    #[test]
    fn timing() {
        let mut checker = HealthChecker::new();
        checker.add_check("slow", || {
            thread::sleep(Duration::from_millis(10));
            Status::Healthy
        });
        let report = checker.check_health();
        assert!(report.checks[0].duration > Duration::ZERO);
    }

    #[test]
    fn empty_checker() {
        let checker = HealthChecker::new();
        let report = checker.check_health();
        assert_eq!(report.status, Status::Healthy);
        assert!(report.checks.is_empty());
    }

    #[test]
    fn status_display() {
        assert_eq!(Status::Healthy.to_string(), "healthy");
        assert_eq!(
            Status::Degraded("slow".into()).to_string(),
            "degraded: slow"
        );
        assert_eq!(
            Status::Unhealthy("down".into()).to_string(),
            "unhealthy: down"
        );
    }
}
