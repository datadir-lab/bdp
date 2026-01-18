# BDP Server API

This directory contains the REST API implementation for the BDP server.

## Structure

- `mod.rs` - API module exports and route registration
- Request handlers for each API endpoint
- Response serialization and error handling
- Middleware integration

## API Endpoints

The API follows RESTful conventions and provides endpoints for:

- Protein queries and searches
- Gene annotations
- Sequence retrieval
- Cross-database linking
- Metadata and version information

## Adding New Endpoints

1. Define the handler function in an appropriate module
2. Add route registration in `mod.rs`
3. Document the endpoint with OpenAPI/Swagger annotations
4. Add integration tests in `tests/`

## Authentication & Authorization

API authentication and authorization logic is configured here. See the security documentation for implementation details.
