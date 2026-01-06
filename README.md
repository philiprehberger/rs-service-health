# rs-service-health

[![CI](https://github.com/philiprehberger/rs-service-health/actions/workflows/ci.yml/badge.svg)](https://github.com/philiprehberger/rs-service-health/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/philiprehberger-service-health.svg)](https://crates.io/crates/philiprehberger-service-health)
[![Last updated](https://img.shields.io/github/last-commit/philiprehberger/rs-service-health)](https://github.com/philiprehberger/rs-service-health/commits/main)

Service health checker with liveness and readiness probes for Rust

## Installation

```toml
[dependencies]
philiprehberger-service-health = "0.1.7"
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

## Development

```bash
cargo test
cargo clippy -- -D warnings
```

## Support

If you find this project useful:

⭐ [Star the repo](https://github.com/philiprehberger/rs-service-health)

🐛 [Report issues](https://github.com/philiprehberger/rs-service-health/issues?q=is%3Aissue+is%3Aopen+label%3Abug)

💡 [Suggest features](https://github.com/philiprehberger/rs-service-health/issues?q=is%3Aissue+is%3Aopen+label%3Aenhancement)

❤️ [Sponsor development](https://github.com/sponsors/philiprehberger)

🌐 [All Open Source Projects](https://philiprehberger.com/open-source-packages)

💻 [GitHub Profile](https://github.com/philiprehberger)

🔗 [LinkedIn Profile](https://www.linkedin.com/in/philiprehberger)

## License

[MIT](LICENSE)
