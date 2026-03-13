# rs-service-health

Service health checker with liveness and readiness probes for Rust. Register health check functions, run them on demand, and get JSON-serializable reports with timing information.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
philiprehberger-service-health = "0.1"
```

## Usage

```rust
use philiprehberger_service_health::{HealthChecker, Status};

let mut checker = HealthChecker::new();

checker.add_liveness_check("database", || {
    // check database connection
    Status::Healthy
});

checker.add_readiness_check("cache", || {
    Status::Degraded("high latency".into())
});

let report = checker.check_health();
println!("{}", report.to_json());
println!("Healthy: {}", report.is_healthy());

// Run only liveness or readiness probes
let liveness = checker.check_liveness();
let readiness = checker.check_readiness();
```

## API

| Type | Description |
|------|-------------|
| `Status` | Health status: `Healthy`, `Degraded(String)`, `Unhealthy(String)` |
| `CheckResult` | Result of a single check with name, status, and duration |
| `HealthReport` | Aggregated report with overall status, check results, and timestamp |
| `HealthChecker` | Registry for health checks with liveness/readiness probe support |

## License

MIT
