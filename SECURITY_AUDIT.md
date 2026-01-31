# Security Audit Report

**Last Updated**: 2026-01-31
**CI Run**: #21534232001

## Summary

This document tracks security vulnerabilities and warnings identified by `cargo audit` and their current status.

### Current Status

- **Critical Vulnerabilities**: 0
- **High Severity Vulnerabilities**: 0
- **Medium Severity Vulnerabilities**: 1 (unfixable)
- **Unmaintained Dependency Warnings**: 3 (transitive dependencies)
- **Unsound Code Warnings**: 1 (transitive dependency)

## Fixed Vulnerabilities

### ✅ RUSTSEC-2025-0111: tokio-tar PAX header parsing vulnerability
- **Status**: FIXED
- **Severity**: N/A (file smuggling vulnerability)
- **Affected Crate**: `tokio-tar 0.3.1`
- **Solution**: Updated `testcontainers` from 0.23.3 to 0.26.3 and `testcontainers-modules` from 0.11.6 to 0.14.0
- **Impact**: Test dependency only (dev-dependencies)
- **Date Fixed**: 2026-01-31

## Unfixable Vulnerabilities

### 🔴 RUSTSEC-2023-0071: Marvin Attack in RSA (Medium Severity)

- **Crate**: `rsa 0.9.10`
- **Severity**: 5.9 (Medium)
- **Status**: No fix available
- **Issue**: Potential key recovery through timing sidechannels (Marvin Attack)
- **URL**: https://rustsec.org/advisories/RUSTSEC-2023-0071
- **Date Reported**: 2023-11-22

**Dependency Chain**:
```
rsa 0.9.10
└── sqlx-mysql 0.8.6
    └── sqlx 0.8.6
        ├── bdp-server
        ├── bdp-ingest
        ├── bdp-cli
        └── apalis-postgres
```

**Analysis**:
- This vulnerability is in the `rsa` crate which is a transitive dependency pulled in by `sqlx-mysql`
- We use SQLx with PostgreSQL as our primary database; MySQL support is included via SQLx's feature flags
- The RSA crate is only used by SQLx for MySQL authentication

**Mitigation**:
1. We do not use MySQL in our production environment (PostgreSQL only)
2. The MySQL-related code paths are not exercised in our application
3. Monitoring upstream for fix in SQLx or rsa crate
4. Consider adding `[patch.crates-io]` if a compatible fork becomes available

**Risk Assessment**: **LOW** - Not used in production code paths

---

## Warnings (Transitive Dependencies)

These are warnings for unmaintained or unsound dependencies that are pulled in transitively. They cannot be directly fixed without upstream updates.

### ⚠️ RUSTSEC-2025-0057: fxhash unmaintained

- **Crate**: `fxhash 0.2.1`
- **Status**: Unmaintained
- **URL**: https://rustsec.org/advisories/RUSTSEC-2025-0057
- **Date Reported**: 2025-09-05

**Dependency Chain**:
```
fxhash 0.2.1
├── selectors 0.26.0
│   └── scraper 0.22.0 (used in bdp-server, bdp-ingest)
└── inquire 0.7.5 (used in bdp-cli, xtask)
```

**Mitigation**: Monitoring upstream `scraper` and `inquire` crates for updates

---

### ⚠️ RUSTSEC-2025-0119: number_prefix unmaintained

- **Crate**: `number_prefix 0.4.0`
- **Status**: Unmaintained
- **URL**: https://rustsec.org/advisories/RUSTSEC-2025-0119
- **Date Reported**: 2025-11-17

**Dependency Chain**:
```
number_prefix 0.4.0
└── indicatif 0.17.11
    ├── bdp-ingest
    └── bdp-cli
```

**Mitigation**: Monitoring upstream `indicatif` crate for updates or replacement

---

### ⚠️ RUSTSEC-2025-0134: rustls-pemfile unmaintained

- **Crate**: `rustls-pemfile 2.2.0`
- **Status**: Unmaintained
- **URL**: https://rustsec.org/advisories/RUSTSEC-2025-0134
- **Date Reported**: 2025-11-28

**Dependency Chain**:
```
rustls-pemfile 2.2.0
└── bollard 0.19.4
    └── testcontainers 0.26.3 (dev-dependency)
```

**Impact**: Test dependency only (dev-dependencies)

**Mitigation**: Monitoring upstream `testcontainers` and `bollard` for updates

---

### ⚠️ RUSTSEC-2026-0002: lru IterMut unsoundness

- **Crate**: `lru 0.12.5`
- **Status**: Unsound code (Stacked Borrows violation)
- **URL**: https://rustsec.org/advisories/RUSTSEC-2026-0002
- **Date Reported**: 2026-01-07

**Dependency Chain**:
```
lru 0.12.5
└── aws-sdk-s3 1.119.0
    └── bdp-server
```

**Issue**: `IterMut` implementation violates Rust's Stacked Borrows memory model

**Mitigation**:
- Monitoring AWS SDK for updates
- The issue is in iterator implementation; we primarily use standard S3 operations
- Low risk as we don't extensively iterate over LRU cache internals

**Risk Assessment**: **LOW** - Limited usage of affected code paths

---

## Fixed Warnings

### ✅ RUSTSEC-2025-0052: async-std discontinued
- **Status**: FIXED
- **Solution**: Updated `suppaftp` from 6.3.0 to 8.0.1 with `tokio-rustls-ring` features
- **Date Fixed**: 2026-01-31

### ✅ RUSTSEC-2024-0375 & RUSTSEC-2021-0145: atty unmaintained/unsound
- **Status**: FIXED
- **Solution**: Removed unused `atty` dependency from bdp-cli
- **Date Fixed**: 2026-01-31

---

## Recommendations

1. **Monitor Upstream Dependencies**:
   - Watch for updates to `sqlx`, `scraper`, `inquire`, `indicatif`, `testcontainers`, and `aws-sdk-s3`
   - Set up automated dependency update checks (e.g., Dependabot)

2. **Periodic Review**:
   - Run `cargo audit` in CI/CD pipeline (already implemented)
   - Review this document monthly for status updates

3. **Consider Alternatives**:
   - If upstream crates remain unmaintained, evaluate alternative libraries
   - For production-critical paths, prioritize dependencies with active maintenance

4. **SQLx MySQL**:
   - Consider disabling MySQL features in SQLx if not needed: `sqlx = { version = "0.8", default-features = false, features = ["runtime-tokio", "postgres", "macros", "uuid", "chrono", "json", "migrate", "bigdecimal"] }`

---

## Audit Command

To reproduce this audit:

```bash
cargo audit
```

To generate a JSON report:

```bash
cargo audit --json > security-audit.json
```

---

## CI Integration

Security audits run automatically on every CI build via the "Security Audit" job. The job configuration can be found in `.github/workflows/ci.yml`.

**Failure Policy**:
- Vulnerabilities (errors): Fail the build
- Warnings: Report but do not fail the build
