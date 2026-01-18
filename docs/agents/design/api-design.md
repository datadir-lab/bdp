# API Design

REST API specification for BDP registry backend.

## Base URL

```
Production:  https://api.bdp.dev/v1
Development: http://localhost:8000/api/v1
```

## Authentication

Most endpoints are public (read-only). Write operations require authentication.

### API Token Authentication

```http
Authorization: Bearer {token}
```

Tokens obtained via:
```http
POST /auth/token
Content-Type: application/json

{
  "username": "researcher@example.com",
  "password": "password"
}
```

Response:
```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIs...",
    "expires_at": "2024-01-17T12:30:45Z"
  }
}
```

## Response Format

### Success Response

```json
{
  "success": true,
  "data": {
    // Response payload
  },
  "meta": {
    // Optional metadata (pagination, etc.)
  }
}
```

### Error Response

```json
{
  "success": false,
  "error": {
    "code": "NOT_FOUND",
    "message": "Data source 'uniprot:P99999' not found",
    "details": {}
  }
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `NOT_FOUND` | 404 | Resource not found |
| `BAD_REQUEST` | 400 | Invalid request parameters |
| `UNAUTHORIZED` | 401 | Missing or invalid authentication |
| `FORBIDDEN` | 403 | Insufficient permissions |
| `CONFLICT` | 409 | Resource already exists |
| `VALIDATION_ERROR` | 422 | Request validation failed |
| `INTERNAL_ERROR` | 500 | Server error |
| `SERVICE_UNAVAILABLE` | 503 | Service temporarily unavailable |

## Endpoints

### Organizations

#### List Organizations

```http
GET /organizations?page=1&limit=20
```

Response:
```json
{
  "success": true,
  "data": [
    {
      "id": "uuid",
      "slug": "uniprot",
      "name": "Universal Protein Resource",
      "website": "https://www.uniprot.org",
      "description": "...",
      "logo_url": "https://cdn.bdp.dev/logos/uniprot.png",
      "is_system": true,
      "entry_count": 567239,
      "created_at": "2024-01-01T00:00:00Z"
    }
  ],
  "meta": {
    "pagination": {
      "page": 1,
      "per_page": 20,
      "total": 15,
      "pages": 1
    }
  }
}
```

#### Get Organization

```http
GET /organizations/:slug
```

Example: `GET /organizations/uniprot`

Response:
```json
{
  "success": true,
  "data": {
    "id": "uuid",
    "slug": "uniprot",
    "name": "Universal Protein Resource",
    "website": "https://www.uniprot.org",
    "description": "...",
    "logo_url": "https://cdn.bdp.dev/logos/uniprot.png",
    "is_system": true,
    "statistics": {
      "data_sources": 567239,
      "tools": 0,
      "total_versions": 5672390,
      "total_downloads": 1234567890
    },
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

### Registry Entries (Unified Search)

#### Search All Entries

```http
GET /search?q={query}&type={type}&organism={organism}&format={format}&page=1&limit=20
```

**Parameters**:
- `q`: Search query (required)
- `type`: Filter by type: `data_source`, `tool` (optional, comma-separated)
- `organism`: Filter by organism (optional, e.g., "human", "mouse")
- `format`: Filter by format (optional, e.g., "fasta", "xml")
- `page`: Page number (default: 1)
- `limit`: Results per page (default: 20, max: 100)

Example: `GET /search?q=insulin&type=data_source&organism=human&format=fasta`

Response:
```json
{
  "success": true,
  "data": {
    "data_sources": [
      {
        "id": "uuid",
        "organization": "uniprot",
        "slug": "P01308",
        "name": "Insulin [Homo sapiens]",
        "description": "Insulin decreases blood glucose concentration...",
        "source_type": "protein",
        "organism": {
          "scientific_name": "Homo sapiens",
          "common_name": "Human",
          "ncbi_taxonomy_id": 9606
        },
        "latest_version": "1.5",
        "external_version": "2025_01",
        "available_formats": ["fasta", "xml", "json"],
        "downloads": 12345,
        "url": "/sources/uniprot/P01308"
      }
    ],
    "tools": [],
    "total": 1
  },
  "meta": {
    "query": "insulin",
    "filters": {
      "type": ["data_source"],
      "organism": "human",
      "format": "fasta"
    },
    "pagination": {
      "page": 1,
      "per_page": 20,
      "total": 1,
      "pages": 1
    }
  }
}
```

### Data Sources

#### List Data Sources

```http
GET /sources?org={organization}&type={type}&organism={organism}&page=1&limit=20
```

Example: `GET /sources?org=uniprot&type=protein&organism=human`

Response:
```json
{
  "success": true,
  "data": [
    {
      "id": "uuid",
      "organization": "uniprot",
      "slug": "P01308",
      "name": "Insulin [Homo sapiens]",
      "source_type": "protein",
      "external_id": "P01308",
      "organism": {
        "scientific_name": "Homo sapiens",
        "common_name": "Human"
      },
      "latest_version": "1.5",
      "downloads": 12345
    }
  ],
  "meta": {
    "pagination": {
      "page": 1,
      "per_page": 20,
      "total": 20438,
      "pages": 1022
    }
  }
}
```

#### Get Data Source

```http
GET /sources/:org/:name
```

Example: `GET /sources/uniprot/P01308`

Response:
```json
{
  "success": true,
  "data": {
    "id": "uuid",
    "organization": {
      "slug": "uniprot",
      "name": "Universal Protein Resource"
    },
    "slug": "P01308",
    "name": "Insulin [Homo sapiens]",
    "description": "Insulin decreases blood glucose concentration...",
    "source_type": "protein",
    "external_id": "P01308",
    "organism": {
      "ncbi_taxonomy_id": 9606,
      "scientific_name": "Homo sapiens",
      "common_name": "Human",
      "rank": "species"
    },
    "protein_metadata": {
      "accession": "P01308",
      "entry_name": "INS_HUMAN",
      "protein_name": "Insulin",
      "gene_name": "INS",
      "sequence_length": 110,
      "mass_da": 11937
    },
    "versions": [
      {
        "version": "1.5",
        "external_version": "2025_01",
        "release_date": "2025-01-15",
        "formats": ["fasta", "xml", "json"],
        "size_bytes": 24576,
        "downloads": 2345
      },
      {
        "version": "1.4",
        "external_version": "2024_12",
        "release_date": "2024-12-10",
        "formats": ["fasta", "xml"],
        "size_bytes": 24320,
        "downloads": 4567
      }
    ],
    "latest_version": "1.5",
    "total_downloads": 12345,
    "tags": ["protein", "hormone", "signaling", "diabetes"],
    "created_at": "2024-01-01T00:00:00Z",
    "updated_at": "2025-01-15T10:30:00Z"
  }
}
```

#### Get Data Source Version

```http
GET /sources/:org/:name/:version
```

Example: `GET /sources/uniprot/P01308/1.5`

Response:
```json
{
  "success": true,
  "data": {
    "id": "uuid",
    "organization": "uniprot",
    "name": "P01308",
    "version": "1.5",
    "external_version": "2025_01",
    "release_date": "2025-01-15",
    "size_bytes": 24576,
    "downloads": 2345,
    "files": [
      {
        "format": "fasta",
        "checksum": "sha256-1a2b3c4d5e6f7890abcdef1234567890abcdef1234567890abcdef1234567890",
        "size_bytes": 4096,
        "compression": "none",
        "download_url": "/sources/uniprot/P01308/1.5/download?format=fasta"
      },
      {
        "format": "xml",
        "checksum": "sha256-9876543210fedcba0987654321fedcba0987654321fedcba0987654321fedcba",
        "size_bytes": 16384,
        "compression": "gzip",
        "download_url": "/sources/uniprot/P01308/1.5/download?format=xml"
      },
      {
        "format": "json",
        "checksum": "sha256-abcd1234efgh5678ijkl9012mnop3456qrst7890uvwx1234yz567890abcdef12",
        "size_bytes": 8192,
        "compression": "none",
        "download_url": "/sources/uniprot/P01308/1.5/download?format=json"
      }
    ],
    "citations": [
      {
        "citation_type": "primary",
        "doi": "10.1093/nar/gkac1052",
        "pubmed_id": "36408920",
        "title": "UniProt: the Universal Protein Knowledgebase in 2023",
        "journal": "Nucleic Acids Research",
        "publication_date": "2023-01-06",
        "authors": "The UniProt Consortium"
      }
    ],
    "has_dependencies": false,
    "dependency_count": 0,
    "published_at": "2025-01-15T10:30:00Z"
  }
}
```

#### Download Data Source File

```http
GET /sources/:org/:name/:version/download?format={format}
```

Example: `GET /sources/uniprot/P01308/1.5/download?format=fasta`

**Response**: Redirects to S3 signed URL or streams file directly

```http
HTTP/1.1 302 Found
Location: https://s3.amazonaws.com/bdp-data/proteins/uniprot/P01308/1.5/P01308.fasta?X-Amz-Signature=...
```

Or direct stream:
```http
HTTP/1.1 200 OK
Content-Type: text/plain
Content-Disposition: attachment; filename="P01308.fasta"
Content-Length: 4096
X-Checksum-SHA256: 1a2b3c4d5e6f7890abcdef1234567890abcdef1234567890abcdef1234567890

>sp|P01308|INS_HUMAN Insulin OS=Homo sapiens OX=9606 GN=INS PE=1 SV=1
MALWMRLLPLLALLALWGPDPAAAFVNQHLCGSHLVEALYLVCGERGFFYTPKTRREAEDLQVGQVELGG
GPGAGSLQPLALEGSLQKRGIVEQCCTSICSLYQLENYCN
```

#### Get Dependencies

```http
GET /sources/:org/:name/:version/dependencies?format={format}&page=1&limit=1000
```

Example: `GET /sources/uniprot/all/1.0/dependencies?format=fasta&page=1&limit=1000`

**For aggregate sources with many dependencies**

Response:
```json
{
  "success": true,
  "data": {
    "source": "uniprot:all@1.0",
    "format": "fasta",
    "dependency_count": 567239,
    "tree_checksum": "sha256-aggregate1234567890abcdef1234567890abcdef1234567890abcdef12345678",
    "dependencies": [
      {
        "source": "uniprot:P01308-fasta@1.0",
        "checksum": "sha256-1a2b3c4d...",
        "size": 4096
      },
      {
        "source": "uniprot:P04637-fasta@1.0",
        "checksum": "sha256-9876543...",
        "size": 8192
      }
      // ... up to 1000 per page
    ]
  },
  "meta": {
    "pagination": {
      "page": 1,
      "per_page": 1000,
      "total": 567239,
      "pages": 568
    }
  }
}
```

### Tools

#### List Tools

```http
GET /tools?org={organization}&type={type}&page=1&limit=20
```

Example: `GET /tools?org=ncbi&type=alignment`

Response:
```json
{
  "success": true,
  "data": [
    {
      "id": "uuid",
      "organization": "ncbi",
      "slug": "blast",
      "name": "BLAST: Basic Local Alignment Search Tool",
      "tool_type": "alignment",
      "latest_version": "2.14.0",
      "downloads": 567890
    }
  ],
  "meta": {
    "pagination": {
      "page": 1,
      "per_page": 20,
      "total": 145,
      "pages": 8
    }
  }
}
```

#### Get Tool

```http
GET /tools/:org/:name
```

Example: `GET /tools/ncbi/blast`

Response:
```json
{
  "success": true,
  "data": {
    "id": "uuid",
    "organization": {
      "slug": "ncbi",
      "name": "National Center for Biotechnology Information"
    },
    "slug": "blast",
    "name": "BLAST: Basic Local Alignment Search Tool",
    "description": "Finds regions of similarity between biological sequences...",
    "tool_type": "alignment",
    "repository_url": "https://github.com/ncbi/blast",
    "homepage_url": "https://blast.ncbi.nlm.nih.gov/",
    "license": "Public Domain",
    "versions": [
      {
        "version": "2.14.0",
        "external_version": "v2.14.0",
        "release_date": "2023-11-15",
        "size_bytes": 104857600,
        "downloads": 12345
      }
    ],
    "latest_version": "2.14.0",
    "total_downloads": 567890,
    "tags": ["alignment", "sequence-analysis", "protein", "nucleotide"]
  }
}
```

#### Download Tool

```http
GET /tools/:org/:name/:version/download
```

Example: `GET /tools/ncbi/blast/2.14.0/download`

**Response**: Redirects to S3 or streams tool archive

### Version Resolution

#### Resolve Version

```http
POST /resolve
Content-Type: application/json

{
  "sources": [
    "uniprot:P01308-fasta@1.0",
    "uniprot:all-fasta@1.0"
  ],
  "tools": [
    "ncbi:blast@2.14.0"
  ]
}
```

Response:
```json
{
  "success": true,
  "data": {
    "sources": {
      "uniprot:P01308-fasta@1.0": {
        "resolved": "uniprot:P01308@1.0",
        "format": "fasta",
        "checksum": "sha256-1a2b3c4d...",
        "size": 4096,
        "external_version": "2025_01",
        "has_dependencies": false
      },
      "uniprot:all-fasta@1.0": {
        "resolved": "uniprot:all@1.0",
        "format": "fasta",
        "checksum": "sha256-aggregate...",
        "size": 4294967296,
        "external_version": "2025_01",
        "has_dependencies": true,
        "dependency_count": 567239
      }
    },
    "tools": {
      "ncbi:blast@2.14.0": {
        "resolved": "ncbi:blast@2.14.0",
        "checksum": "sha256-blast123...",
        "size": 104857600
      }
    }
  }
}
```

### Statistics

#### Get Global Statistics

```http
GET /stats
```

Response:
```json
{
  "success": true,
  "data": {
    "organizations": 15,
    "data_sources": 567239,
    "tools": 145,
    "total_versions": 5672535,
    "total_downloads": 123456789,
    "total_size_bytes": 5497558138880,
    "top_downloads": [
      {
        "source": "uniprot:all-fasta@1.0",
        "downloads": 5678
      },
      {
        "source": "ncbi:GRCh38-fasta@2.0",
        "downloads": 4321
      }
    ]
  }
}
```

### Publishing (Authenticated)

#### Publish Data Source

```http
POST /sources
Authorization: Bearer {token}
Content-Type: multipart/form-data

{
  "organization": "my-lab",
  "name": "custom-protein-set",
  "version": "1.0",
  "source_type": "protein",
  "description": "Curated protein set for cancer research",
  "format": "fasta",
  "file": <binary>
}
```

Response:
```json
{
  "success": true,
  "data": {
    "id": "uuid",
    "organization": "my-lab",
    "slug": "custom-protein-set",
    "version": "1.0",
    "checksum": "sha256-computed...",
    "size_bytes": 12345,
    "url": "/sources/my-lab/custom-protein-set"
  }
}
```

## Rate Limiting

```http
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1642348800
```

**Limits**:
- Anonymous: 100 requests/hour
- Authenticated: 1000 requests/hour
- Downloads: 100 GB/day per IP

**When exceeded**:
```http
HTTP/1.1 429 Too Many Requests
Retry-After: 3600

{
  "success": false,
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "Rate limit exceeded. Try again in 3600 seconds.",
    "details": {
      "limit": 1000,
      "reset_at": "2024-01-16T14:00:00Z"
    }
  }
}
```

## Pagination

All list endpoints support pagination:

```http
GET /sources?page=2&limit=50
```

Response includes pagination metadata:
```json
{
  "meta": {
    "pagination": {
      "page": 2,
      "per_page": 50,
      "total": 567239,
      "pages": 11345,
      "has_next": true,
      "has_prev": true
    }
  }
}
```

**Limits**:
- Default `limit`: 20
- Max `limit`: 100
- Max `page`: 10000 (use filters for deeper pagination)

## Filtering & Sorting

### Filtering

```http
GET /sources?org=uniprot&type=protein&organism=human
```

**Supported Filters**:
- `org`: Organization slug
- `type`: Source type or tool type
- `organism`: Organism name or taxonomy ID
- `format`: File format
- `tag`: Tag name (repeatable)

### Sorting

```http
GET /sources?sort=-downloads,name
```

**Format**: `sort={field}` or `sort=-{field}` (descending)

**Supported Fields**:
- `name`: Alphabetical
- `downloads`: Download count
- `created_at`: Creation date
- `updated_at`: Last update date
- `size`: Size in bytes

## Caching

Responses include cache headers:

```http
Cache-Control: public, max-age=3600
ETag: "abc123def456"
Last-Modified: Wed, 15 Jan 2025 10:30:00 GMT
```

**Conditional Requests**:
```http
GET /sources/uniprot/P01308
If-None-Match: "abc123def456"
```

Response if unchanged:
```http
HTTP/1.1 304 Not Modified
```

## CORS

CORS enabled for all origins:

```http
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS
Access-Control-Allow-Headers: Content-Type, Authorization
```

## Health Check

```http
GET /health
```

Response:
```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime": 864000,
  "database": "connected",
  "storage": "available"
}
```

## WebSocket (Future)

For real-time updates on long-running operations:

```javascript
const ws = new WebSocket('wss://api.bdp.dev/v1/ws');

ws.send(JSON.stringify({
  action: 'subscribe',
  channel: 'source_updates',
  filter: { organization: 'uniprot' }
}));

ws.onmessage = (event) => {
  const update = JSON.parse(event.data);
  console.log('New version:', update);
};
```

## Related Documents

- [Database Schema](./database-schema.md) - Backend data model
- [File Formats](./file-formats.md) - bdp.yml and bdl.lock
- [Cache Strategy](./cache-strategy.md) - Client-side caching
- [Dependency Resolution](./dependency-resolution.md) - Resolution logic
