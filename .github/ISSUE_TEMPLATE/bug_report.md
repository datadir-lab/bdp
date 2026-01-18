---
name: Bug Report
about: Create a report to help us improve BDP
title: '[BUG] '
labels: bug
assignees: ''
---

## Bug Description

A clear and concise description of what the bug is.

## To Reproduce

Steps to reproduce the behavior:

1. Run command '...'
2. With configuration '...'
3. See error

## Expected Behavior

A clear and concise description of what you expected to happen.

## Actual Behavior

What actually happened instead.

## Environment

**CLI Version:**
```bash
bdp --version
```

**Operating System:**
- [ ] Linux (distro and version: ___)
- [ ] macOS (version: ___)
- [ ] Windows (version: ___)

**Rust Version:**
```bash
rustc --version
```

**Database:**
- PostgreSQL version:
- Connection method: (local/remote)

## Configuration

**bdp.yml** (if relevant):
```yaml
# Paste your bdp.yml here
```

**.env** (redact sensitive info):
```env
# Paste relevant environment variables here
```

## Logs

**Error output:**
```
Paste the full error message or logs here
```

**API Server logs** (if relevant):
```
Paste server logs here
```

**CLI output with debug flag:**
```bash
RUST_LOG=debug bdp <command>
# Paste output here
```

## Stack Trace

If applicable, paste the full stack trace here:
```
```

## Screenshots

If applicable, add screenshots to help explain your problem.

## Additional Context

Add any other context about the problem here:

- Are you using a team cache?
- Are you behind a proxy/firewall?
- Is this reproducible on a fresh installation?
- Did this work in a previous version?

## Possible Solution

If you have any ideas on how to fix this, please share them here.

## Related Issues

Link any related issues here:
- #issue_number
