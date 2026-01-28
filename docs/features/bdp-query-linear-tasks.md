# BDP Query - Linear Task Breakdown

## Epic: BDP Query Command Implementation

**Description:** Implement advanced SQL-like querying interface for BDP data sources with entity aliases, auto-join metadata, and multiple output formats.

**Labels:** `feature`, `cli`, `priority-high`, `quarter-q1-2026`

---

## Phase 1: Core Functionality (Sprint 1-2)

### Task 1: CLI Command Structure & Flag Parsing
**Estimate:** 5 points
**Description:**
- Add `bdp query` command to CLI with clap
- Implement flag parsing for all Phase 1 flags
- Add `--select`, `--where`, `--order-by`, `--limit`, `--offset`
- Add `--format`, `--output`, `--no-header`
- Add `--sql`, `--dry-run`, `--explain`
- Unit tests for flag parsing

**Acceptance Criteria:**
- [ ] `bdp query --help` shows all flags
- [ ] All flags parse correctly
- [ ] Invalid flags show helpful errors
- [ ] Unit tests pass

---

### Task 2: Entity Alias Resolution
**Estimate:** 3 points
**Description:**
- Implement entity alias mapping (protein → data_sources WHERE type='protein')
- Support: protein, gene, genome, transcriptome, proteome, tools, orgs
- Add alias validation and error messages
- Unit tests for all aliases

**Acceptance Criteria:**
- [ ] `bdp query protein` resolves correctly
- [ ] All aliases map to correct tables
- [ ] Invalid alias shows error: "Unknown entity: xyz"
- [ ] Unit tests cover all aliases

---

### Task 3: SQL Query Builder
**Estimate:** 8 points
**Description:**
- Integrate `sqlparser-rs` for SQL parsing
- Build SQL from flags (--where, --select, --order-by, etc.)
- Handle simple WHERE conditions (key=value)
- Handle complex WHERE expressions (with AND, OR, operators)
- Generate proper SQL with parameters
- Unit tests for query building

**Acceptance Criteria:**
- [ ] Simple flags build correct SQL
- [ ] Complex WHERE expressions parse
- [ ] SQL injection is prevented
- [ ] Generated SQL is valid PostgreSQL
- [ ] Unit tests cover edge cases

---

### Task 4: Backend API Endpoint
**Estimate:** 8 points
**Description:**
- Create `/api/v1/query` POST endpoint
- Accept SQL query with parameters
- Validate SQL syntax
- Check user permissions
- Execute query against PostgreSQL
- Return paginated results
- Add query timeout (30s)

**Acceptance Criteria:**
- [ ] Endpoint accepts SQL queries
- [ ] Invalid SQL returns 400 error
- [ ] Permission checks work
- [ ] Results are paginated
- [ ] Timeout prevents long queries
- [ ] Integration tests pass

---

### Task 5: Output Formatters
**Estimate:** 5 points
**Description:**
- Implement table formatter (comfy-table)
- Implement JSON formatter (serde_json)
- Implement CSV formatter
- Implement TSV formatter
- Implement compact formatter
- Add --no-header support for CSV/TSV
- Add --output file writing

**Acceptance Criteria:**
- [ ] All formats produce correct output
- [ ] Table format is human-readable
- [ ] JSON is valid and pretty-printed
- [ ] CSV/TSV work with --no-header
- [ ] Output writes to file with --output
- [ ] Unit tests for each formatter

---

### Task 6: Smart TTY Detection
**Estimate:** 2 points
**Description:**
- Detect if stdout is a TTY
- Default to `table` format for TTY
- Default to `tsv` format for pipes
- Disable colors when piped
- Add progress indicators for TTY only

**Acceptance Criteria:**
- [ ] TTY detection works correctly
- [ ] Interactive mode uses table format
- [ ] Piped mode uses TSV format
- [ ] Colors disabled when piped
- [ ] Progress shows only in TTY

---

### Task 7: Error Handling & Messages
**Estimate:** 3 points
**Description:**
- Implement simplified error messages
- Add detailed errors with --verbose
- Show helpful hints for common errors
- Format SQL errors nicely
- Add suggestions for typos

**Acceptance Criteria:**
- [ ] Errors are clear and actionable
- [ ] --verbose shows detailed info
- [ ] Common errors have hints
- [ ] No raw SQL errors shown (unless --verbose)
- [ ] Error messages tested

---

### Task 8: E2E Tests for Phase 1
**Estimate:** 5 points
**Description:**
- Write E2E tests with testcontainers
- Test all entity aliases
- Test all output formats
- Test simple and complex queries
- Test error cases
- Test piping workflow

**Acceptance Criteria:**
- [ ] 20+ E2E tests passing
- [ ] All aliases tested
- [ ] All formats tested
- [ ] Error cases covered
- [ ] Piping scenarios tested

---

### Task 9: Documentation for Phase 1
**Estimate:** 2 points
**Description:**
- Update CLI help text
- Write user guide for `bdp query`
- Add examples to docs
- Update README with query examples
- Document all flags

**Acceptance Criteria:**
- [ ] Help text is complete
- [ ] User guide covers all features
- [ ] 10+ examples in docs
- [ ] README updated
- [ ] All flags documented

---

## Phase 2: Advanced Features (Sprint 3-4)

### Task 10: Auto-Join Metadata Tables
**Estimate:** 8 points
**Description:**
- Implement auto-join logic for entity aliases
- protein → LEFT JOIN protein_metadata
- gene → LEFT JOIN gene_metadata
- Auto-join organism_taxonomy when available
- Auto-join publication_refs when available
- Make auto-join toggleable with flag

**Acceptance Criteria:**
- [ ] Metadata auto-joins work for all types
- [ ] No duplicate columns in results
- [ ] Can disable auto-join with --no-auto-join
- [ ] Performance impact is minimal
- [ ] Tests cover all join scenarios

---

### Task 11: Complex WHERE Expressions
**Estimate:** 5 points
**Description:**
- Support full SQL WHERE syntax
- AND, OR, NOT operators
- IN, LIKE, BETWEEN operators
- Comparison operators (>, <, >=, <=, !=)
- Handle nested expressions
- Proper operator precedence

**Acceptance Criteria:**
- [ ] All SQL operators work
- [ ] Nested expressions parse correctly
- [ ] Operator precedence is correct
- [ ] Complex queries execute properly
- [ ] Unit tests for all operators

---

### Task 12: Aggregation Support
**Estimate:** 8 points
**Description:**
- Implement --group-by flag
- Implement --aggregate flag
- Support COUNT, SUM, AVG, MIN, MAX
- Implement --having flag
- Handle multiple aggregations
- Test with complex GROUP BY

**Acceptance Criteria:**
- [ ] GROUP BY works correctly
- [ ] All aggregate functions work
- [ ] HAVING filters grouped results
- [ ] Multiple aggregations supported
- [ ] Tests cover aggregation cases

---

### Task 13: JOIN Support
**Estimate:** 8 points
**Description:**
- Implement --join flag
- Implement --on flag
- Support INNER, LEFT, RIGHT, FULL joins
- Handle multiple joins
- Validate join conditions
- Test complex join scenarios

**Acceptance Criteria:**
- [ ] Basic joins work
- [ ] All join types supported
- [ ] Multiple joins possible
- [ ] Join conditions validated
- [ ] Tests cover join scenarios

---

### Task 14: Query Debugging Tools
**Estimate:** 3 points
**Description:**
- Implement --explain flag (show EXPLAIN output)
- Implement --dry-run (show SQL without executing)
- Add query timing information
- Show row count estimates
- Format EXPLAIN output nicely

**Acceptance Criteria:**
- [ ] --explain shows query plan
- [ ] --dry-run shows SQL only
- [ ] Timing info accurate
- [ ] EXPLAIN output readable
- [ ] Tests for debugging tools

---

### Task 15: Progress Indicators
**Estimate:** 3 points
**Description:**
- Add progress bar for long queries
- Show elapsed time
- Show estimated completion
- Handle query cancellation (Ctrl+C)
- Only show in TTY mode

**Acceptance Criteria:**
- [ ] Progress shows for queries >1s
- [ ] Progress bar updates smoothly
- [ ] Ctrl+C cancels query
- [ ] Only shows in TTY
- [ ] Tests for progress logic

---

## Phase 3: Query Management (Sprint 5-6)

### Task 16: Query History
**Estimate:** 8 points
**Description:**
- Store query history in SQLite
- Implement --history flag
- Implement --history-run <n>
- Implement --history-search
- Implement --history-clear
- Limit history to 1000 queries

**Acceptance Criteria:**
- [ ] Queries saved to history
- [ ] --history shows last 20 queries
- [ ] Can re-run from history
- [ ] Search works
- [ ] History limited to 1000

---

### Task 17: Saved Queries
**Estimate:** 8 points
**Description:**
- Implement --save flag
- Implement --load flag
- Implement --list-saved
- Implement --delete-saved
- Store in ~/.bdp/saved_queries.yaml
- Support query parameters

**Acceptance Criteria:**
- [ ] Can save queries
- [ ] Can load saved queries
- [ ] List shows all saved queries
- [ ] Can delete saved queries
- [ ] Parameters work in saved queries

---

### Task 18: Query Templates
**Estimate:** 8 points
**Description:**
- Implement template system
- Implement --template flag
- Add built-in templates (popular_datasets, recent_updates, etc.)
- Support template parameters
- Allow custom template creation
- Document template syntax

**Acceptance Criteria:**
- [ ] 5+ built-in templates work
- [ ] Can create custom templates
- [ ] Parameters work in templates
- [ ] --list-templates shows all
- [ ] Documentation complete

---

### Task 19: Query Export/Import
**Estimate:** 3 points
**Description:**
- Implement --export-saved
- Implement --import-saved
- Support YAML format
- Handle version compatibility
- Add migration for schema changes

**Acceptance Criteria:**
- [ ] Export creates valid YAML
- [ ] Import loads queries correctly
- [ ] Version compatibility checked
- [ ] Migration handles schema changes
- [ ] Tests for export/import

---

## Phase 4: Optimization & Polish (Sprint 7)

### Task 20: Query Result Caching
**Estimate:** 5 points
**Description:**
- Implement backend query caching
- Use Redis or in-memory cache
- 5 minute TTL by default
- Cache key from SQL hash
- Add --no-cache flag

**Acceptance Criteria:**
- [ ] Query results cached
- [ ] Cache hit improves performance
- [ ] TTL configurable
- [ ] --no-cache bypasses cache
- [ ] Cache stats available

---

### Task 21: Performance Optimization
**Estimate:** 5 points
**Description:**
- Optimize SQL generation
- Add query plan hints
- Implement connection pooling
- Add query result streaming for large results
- Profile and optimize hot paths

**Acceptance Criteria:**
- [ ] Simple queries < 100ms
- [ ] Complex queries < 1s
- [ ] Large results stream efficiently
- [ ] Connection pooling works
- [ ] Performance benchmarks pass

---

### Task 22: Query Analytics
**Estimate:** 5 points
**Description:**
- Track query performance metrics
- Log slow queries
- Add query stats endpoint
- Dashboard for query analytics
- Alert on performance degradation

**Acceptance Criteria:**
- [ ] Metrics tracked for all queries
- [ ] Slow queries logged
- [ ] Stats endpoint works
- [ ] Dashboard shows metrics
- [ ] Alerts configured

---

### Task 23: Final Documentation & Examples
**Estimate:** 3 points
**Description:**
- Complete user guide
- Add 50+ query examples
- Create video tutorials
- Update API docs
- Write blog post

**Acceptance Criteria:**
- [ ] User guide complete
- [ ] 50+ examples documented
- [ ] 3+ video tutorials created
- [ ] API docs updated
- [ ] Blog post published

---

## Total Estimates

**Phase 1:** 41 points (2 sprints)
**Phase 2:** 35 points (2 sprints)
**Phase 3:** 27 points (2 sprints)
**Phase 4:** 18 points (1 sprint)

**Total:** 121 points (~7 sprints / ~3.5 months)

---

## Dependencies

- `sqlparser-rs` - SQL parsing
- `comfy-table` - Table formatting
- `serde_json` - JSON output
- `csv` - CSV/TSV output
- `rusqlite` - Query history storage
- `redis` (optional) - Query caching

---

## Risk & Mitigation

**Risk:** SQL injection vulnerabilities
**Mitigation:** Always use parameterized queries, thorough security testing

**Risk:** Performance issues with large result sets
**Mitigation:** Implement streaming, pagination, result limits

**Risk:** Complex SQL parsing edge cases
**Mitigation:** Extensive testing, fallback to --sql for complex cases

**Risk:** User confusion between search and query
**Mitigation:** Clear documentation, examples, helpful error messages
