-- Initial schema setup with UUID extension
-- This migration enables the gen_random_uuid() function for PostgreSQL

CREATE EXTENSION IF NOT EXISTS "pgcrypto";
