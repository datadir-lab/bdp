-- Widen varchar columns that were too narrow for real UniProt data
ALTER TABLE protein_features ALTER COLUMN feature_type TYPE varchar(100);
ALTER TABLE protein_cross_references ALTER COLUMN database TYPE varchar(100);
ALTER TABLE protein_signatures ALTER COLUMN database TYPE varchar(100);
ALTER TABLE protein_signatures ALTER COLUMN accession TYPE varchar(100);
ALTER TABLE protein_signatures ALTER COLUMN clan_accession TYPE varchar(100);
ALTER TABLE registry_entries ALTER COLUMN name TYPE varchar(500);
