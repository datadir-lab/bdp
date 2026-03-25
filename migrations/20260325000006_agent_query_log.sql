-- migrations/20260325000006_agent_query_log.sql
-- MCP agent query provenance log.
-- Records every tool call an AI agent makes for auditing and debugging.

CREATE TABLE agent_query_log (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id         TEXT,
    tool_name        TEXT NOT NULL,
    query_params     JSONB NOT NULL,
    dataset_versions JSONB NOT NULL,
    result_count     INTEGER,
    duration_ms      INTEGER,
    executed_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_agent_log_agent ON agent_query_log(agent_id, executed_at);
CREATE INDEX idx_agent_log_tool  ON agent_query_log(tool_name, executed_at);
