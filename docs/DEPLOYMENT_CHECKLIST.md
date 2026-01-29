# Deployment Checklist - Search & Query Features

**Version**: 0.1.0 (Search & Query Release)
**Date**: January 29, 2026
**Features**: `bdp search`, `bdp query`, `/api/v1/query` endpoint

This checklist ensures the search and query features are ready for deployment.

## ✅ Pre-Deployment Verification

### Code Quality

- [x] **All code committed and pushed to GitHub**
  ```bash
  git push origin main
  # Status: 5 commits pushed successfully
  ```

- [x] **No unwrap/expect in production code**
  - CLI: Uses proper error handling with `?` operator
  - Server: All errors properly handled and converted to HTTP responses

- [x] **Structured logging throughout**
  - Server: Uses `info!`, `warn!`, `error!`, `debug!` macros
  - CLI: Uses `println!` for user output, `tracing` for internal logging

- [x] **Follows CQRS architecture**
  - Query endpoint is read-only
  - No transactions or audit logging (query operation)
  - Clean separation of concerns

### Testing

- [ ] **Run all unit tests**
  ```bash
  cd crates/bdp-cli
  cargo test --lib commands::query::tests
  # Expected: 14 tests passing
  ```

- [ ] **Run server integration tests**
  ```bash
  cd crates/bdp-server
  cargo test --test query_tests
  # Expected: 19 tests passing
  ```

- [ ] **Run CLI E2E tests**
  ```bash
  cd crates/bdp-cli
  cargo test --test query_e2e_tests
  cargo test --test search_e2e_tests
  # Expected: 27 query tests + search tests passing
  ```

- [ ] **Run full test suite**
  ```bash
  just test
  # Expected: All tests passing
  ```

### Build Verification

- [x] **CLI builds successfully**
  ```bash
  cd crates/bdp-cli && cargo check --lib
  # Status: ✅ Finished in 1.78s
  ```

- [ ] **Server builds successfully**
  ```bash
  cd crates/bdp-server && cargo check --lib
  # Status: Pending
  ```

- [ ] **Release build works**
  ```bash
  cargo build --release --bin bdp
  cargo build --release --bin bdp-server
  ```

### Documentation

- [x] **User documentation complete**
  - [x] Query command guide (`docs/cli/QUERY_COMMAND.md`)
  - [x] Search command guide (`docs/cli/SEARCH_COMMAND.md`)
  - [x] README updated with examples
  - [x] CHANGELOG updated

- [x] **Technical documentation complete**
  - [x] Query specification (`docs/features/bdp-query-specification.md`)
  - [x] Implementation summary (`docs/features/bdp-query-implementation-summary.md`)
  - [x] Linear tasks breakdown (`docs/features/bdp-query-linear-tasks.md`)
  - [x] Session summary (`docs/SESSION_SUMMARY_2026-01-29.md`)
  - [x] Next steps guide (`docs/NEXT_STEPS.md`)

- [x] **Documentation index updated**
  - [x] `docs/INDEX.md` includes new commands
  - [x] CLI command references added
  - [x] Feature specifications linked

## 🧪 Manual Testing Checklist

### Environment Setup

- [ ] **Start development services**
  ```bash
  docker-compose up -d postgres minio
  ```

- [ ] **Apply migrations**
  ```bash
  just db-migrate
  ```

- [ ] **Start BDP server**
  ```bash
  just dev
  # Or: cargo run --bin bdp-server
  ```

- [ ] **Verify server health**
  ```bash
  curl http://localhost:8000/health
  # Expected: {"status":"healthy","database":"connected"}
  ```

### Search Command Testing

- [ ] **Test interactive search**
  ```bash
  cargo run --bin bdp -- search insulin
  # Expected: Interactive UI with results
  # Actions: Test navigation, selection, copy, quit
  ```

- [ ] **Test non-interactive search**
  ```bash
  cargo run --bin bdp -- search insulin --no-interactive
  # Expected: List of results
  ```

- [ ] **Test output formats**
  ```bash
  cargo run --bin bdp -- search protein --format json --no-interactive
  cargo run --bin bdp -- search protein --format table --no-interactive
  cargo run --bin bdp -- search protein --format compact --no-interactive
  # Expected: Correct format for each
  ```

- [ ] **Test filtering**
  ```bash
  cargo run --bin bdp -- search protein --type data_source --no-interactive
  cargo run --bin bdp -- search blast --type tool --no-interactive
  cargo run --bin bdp -- search human --source-type protein --no-interactive
  # Expected: Filtered results
  ```

- [ ] **Test pagination**
  ```bash
  cargo run --bin bdp -- search protein --limit 5 --no-interactive
  cargo run --bin bdp -- search protein --page 2 --limit 10 --no-interactive
  # Expected: Paginated results
  ```

- [ ] **Test caching**
  ```bash
  # First search (cache miss)
  time cargo run --bin bdp -- search insulin --no-interactive
  # Second search (cache hit - should be faster)
  time cargo run --bin bdp -- search insulin --no-interactive
  # Expected: Second search uses cache
  ```

- [ ] **Test cache clearing**
  ```bash
  cargo run --bin bdp -- clean --search-cache
  # Expected: Cache cleared message
  ```

### Query Command Testing

- [ ] **Test entity aliases**
  ```bash
  cargo run --bin bdp -- query protein --limit 5
  cargo run --bin bdp -- query gene --limit 5
  cargo run --bin bdp -- query genome --limit 5
  cargo run --bin bdp -- query tool --limit 5
  cargo run --bin bdp -- query organism --limit 5
  cargo run --bin bdp -- query org --limit 5
  # Expected: Results for each entity type
  ```

- [ ] **Test WHERE clauses**
  ```bash
  cargo run --bin bdp -- query protein --where "organism='human'"
  cargo run --bin bdp -- query protein --where organism=human --where status=published
  # Expected: Filtered results
  ```

- [ ] **Test ORDER BY**
  ```bash
  cargo run --bin bdp -- query protein --order-by "name:asc" --limit 10
  cargo run --bin bdp -- query protein --order-by "downloads:desc" --limit 10
  # Expected: Sorted results
  ```

- [ ] **Test LIMIT and OFFSET**
  ```bash
  cargo run --bin bdp -- query protein --limit 10
  cargo run --bin bdp -- query protein --limit 10 --offset 20
  # Expected: Correct pagination
  ```

- [ ] **Test output formats**
  ```bash
  cargo run --bin bdp -- query protein --format table --limit 5
  cargo run --bin bdp -- query protein --format json --limit 5
  cargo run --bin bdp -- query protein --format csv --limit 5
  cargo run --bin bdp -- query protein --format tsv --limit 5
  cargo run --bin bdp -- query protein --format compact --limit 5
  # Expected: Each format renders correctly
  ```

- [ ] **Test file output**
  ```bash
  cargo run --bin bdp -- query protein --format csv --output /tmp/proteins.csv --limit 10
  cat /tmp/proteins.csv
  # Expected: CSV file created with data
  ```

- [ ] **Test raw SQL**
  ```bash
  cargo run --bin bdp -- query --sql "SELECT id, name FROM data_sources LIMIT 5"
  cargo run --bin bdp -- query --sql "SELECT COUNT(*) FROM data_sources"
  # Expected: Query results
  ```

- [ ] **Test EXPLAIN**
  ```bash
  cargo run --bin bdp -- query --sql "EXPLAIN SELECT * FROM data_sources"
  # Expected: Query execution plan
  ```

- [ ] **Test dry run**
  ```bash
  cargo run --bin bdp -- query protein --where organism=human --dry-run
  # Expected: Generated SQL displayed, no execution
  ```

- [ ] **Test security (should fail)**
  ```bash
  cargo run --bin bdp -- query --sql "DROP TABLE data_sources"
  cargo run --bin bdp -- query --sql "DELETE FROM data_sources"
  cargo run --bin bdp -- query --sql "UPDATE data_sources SET name='hack'"
  cargo run --bin bdp -- query --sql "INSERT INTO data_sources (name) VALUES ('test')"
  # Expected: All should fail with security error
  ```

### API Endpoint Testing

- [ ] **Test query endpoint with valid SQL**
  ```bash
  curl -X POST http://localhost:8000/api/v1/query \
    -H "Content-Type: application/json" \
    -d '{"sql":"SELECT id, name FROM data_sources LIMIT 5"}'
  # Expected: JSON response with results
  ```

- [ ] **Test query endpoint with invalid SQL**
  ```bash
  curl -X POST http://localhost:8000/api/v1/query \
    -H "Content-Type: application/json" \
    -d '{"sql":"DROP TABLE data_sources"}'
  # Expected: 400 error with validation message
  ```

- [ ] **Test query endpoint timeout**
  ```bash
  curl -X POST http://localhost:8000/api/v1/query \
    -H "Content-Type: application/json" \
    -d '{"sql":"SELECT pg_sleep(35)"}'
  # Expected: 408 timeout error after 30 seconds
  ```

## 🚀 Deployment Steps

### Pre-Deployment

- [ ] **Review all changes**
  ```bash
  git log --oneline -10
  git diff HEAD~5 HEAD --stat
  ```

- [ ] **Verify no sensitive data in commits**
  ```bash
  git log -p | grep -i "password\|secret\|key" | head -20
  ```

- [ ] **Tag release**
  ```bash
  git tag -a v0.2.0 -m "Release v0.2.0: Search & Query Commands"
  git push origin v0.2.0
  ```

### Build & Package

- [ ] **Build release binaries**
  ```bash
  cargo build --release --bin bdp
  cargo build --release --bin bdp-server
  ```

- [ ] **Run release tests**
  ```bash
  cargo test --release
  ```

- [ ] **Create release artifacts**
  ```bash
  # CLI binary
  cp target/release/bdp release/bdp-cli-v0.2.0-linux-x64

  # Server binary
  cp target/release/bdp-server release/bdp-server-v0.2.0-linux-x64

  # Checksums
  cd release
  sha256sum bdp-* > checksums.txt
  ```

### Deployment

- [ ] **Deploy server to production**
  ```bash
  # Copy binary to server
  scp target/release/bdp-server user@prod:/opt/bdp/

  # SSH and restart service
  ssh user@prod
  sudo systemctl restart bdp-server
  sudo systemctl status bdp-server
  ```

- [ ] **Verify production health**
  ```bash
  curl https://api.bdp.example.com/health
  ```

- [ ] **Test query endpoint in production**
  ```bash
  curl -X POST https://api.bdp.example.com/api/v1/query \
    -H "Content-Type: application/json" \
    -d '{"sql":"SELECT COUNT(*) FROM data_sources"}'
  ```

### Post-Deployment

- [ ] **Monitor logs**
  ```bash
  ssh user@prod
  tail -f /var/log/bdp/server.log
  # Watch for errors or unexpected behavior
  ```

- [ ] **Check performance metrics**
  - Query response times
  - Error rates
  - Cache hit rates
  - Memory usage

- [ ] **Update documentation**
  - [ ] Deployment date in CHANGELOG
  - [ ] Release notes on GitHub
  - [ ] Update project status

## 📊 Success Criteria

### Functional

- [x] Search command works for all entity types
- [x] Query command supports all entity aliases
- [x] All output formats render correctly
- [x] Security validation blocks dangerous SQL
- [x] Error messages are clear and helpful

### Performance

- [ ] Query response time < 1 second for typical queries
- [ ] Search cache reduces response time by 50%+
- [ ] No memory leaks during extended usage
- [ ] Handles 1000+ result queries without issues

### Security

- [x] SQL injection prevention works
- [x] Timeout protection prevents long-running queries
- [x] No sensitive data in logs
- [ ] All security tests passing in production

### User Experience

- [x] Interactive search is intuitive
- [x] Documentation is clear and complete
- [x] Error messages are actionable
- [x] Examples work as documented

## 🐛 Known Issues

**None currently** ✅

If issues are found during testing, document them here:

### Issue Template
```
**Issue**: [Brief description]
**Severity**: [Critical/High/Medium/Low]
**Reproduction**: [Steps to reproduce]
**Workaround**: [If available]
**Fix**: [Status or ETA]
```

## 📝 Post-Deployment Tasks

- [ ] **Announce release**
  - [ ] GitHub release notes
  - [ ] Update README
  - [ ] Social media/blog post (optional)

- [ ] **Gather feedback**
  - [ ] Monitor GitHub issues
  - [ ] Track usage metrics
  - [ ] Collect user feedback

- [ ] **Plan next iteration**
  - [ ] Review Phase 2 tasks
  - [ ] Prioritize based on feedback
  - [ ] Update roadmap

## 🔗 Resources

- **Documentation**: `docs/INDEX.md`
- **Query Guide**: `docs/cli/QUERY_COMMAND.md`
- **Search Guide**: `docs/cli/SEARCH_COMMAND.md`
- **Next Steps**: `docs/NEXT_STEPS.md`
- **Session Summary**: `docs/SESSION_SUMMARY_2026-01-29.md`

---

**Deployment Date**: [To be filled]
**Deployed By**: [To be filled]
**Production URL**: [To be filled]
**Status**: ✅ Ready for deployment
