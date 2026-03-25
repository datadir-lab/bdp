-- ClinicalTrials.gov
CREATE TABLE clinical_trials (
    id              BIGSERIAL PRIMARY KEY,
    nct_id          TEXT NOT NULL UNIQUE,
    title           TEXT,
    status          TEXT,
    phase           TEXT,
    start_date      DATE,
    completion_date DATE,
    sponsor         TEXT,
    source_version  TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX ON clinical_trials(status);
CREATE INDEX ON clinical_trials(nct_id);

CREATE TABLE trial_disease_links (
    id              BIGSERIAL PRIMARY KEY,
    trial_id        BIGINT NOT NULL REFERENCES clinical_trials(id),
    disease_term_id UUID REFERENCES disease_terms(id),
    raw_condition   TEXT NOT NULL,
    UNIQUE(trial_id, raw_condition)
);
CREATE INDEX ON trial_disease_links(trial_id);
CREATE INDEX ON trial_disease_links(disease_term_id) WHERE disease_term_id IS NOT NULL;

CREATE TABLE trial_intervention_links (
    id          BIGSERIAL PRIMARY KEY,
    trial_id    BIGINT NOT NULL REFERENCES clinical_trials(id),
    compound_id UUID REFERENCES data_sources(id),
    raw_name    TEXT NOT NULL
);
CREATE INDEX ON trial_intervention_links(trial_id);
CREATE INDEX ON trial_intervention_links(compound_id) WHERE compound_id IS NOT NULL;
