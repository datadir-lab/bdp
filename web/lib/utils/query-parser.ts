/**
 * Query Parser for BDP Search
 *
 * Parses special search formats like:
 * - uniprot:P → organization: uniprot, slug: P
 * - uniprot:swissprot → organization: uniprot, slug: swissprot
 * - uniprot:P@1.0 → organization: uniprot, slug: P, version: 1.0
 * - uniprot:P-fasta → organization: uniprot, slug: P, format: fasta
 * - uniprot:P@1.0-fasta → organization: uniprot, slug: P, version: 1.0, format: fasta
 */

export interface ParsedQuery {
  /** The original query string */
  original: string;
  /** Whether the query matched a special format */
  isSpecialFormat: boolean;
  /** The plain search query (if no special format) */
  plainQuery?: string;
  /** The organization slug (from org:slug format) */
  organization?: string;
  /** The data source slug */
  slug?: string;
  /** The version specifier (@version) */
  version?: string;
  /** The format specifier (-format) */
  format?: string;
  /** Bundle identifier (e.g., swissprot in uniprot:swissprot) */
  bundle?: string;
}

/**
 * Regular expression to match special query formats:
 * Format: org:slug[@version][-format]
 *
 * Examples:
 * - uniprot:P → org=uniprot, slug=P
 * - uniprot:P@1.0 → org=uniprot, slug=P, version=1.0
 * - uniprot:P-fasta → org=uniprot, slug=P, format=fasta
 * - uniprot:P@1.0-fasta → org=uniprot, slug=P, version=1.0, format=fasta
 * - uniprot:swissprot → org=uniprot, slug=swissprot (could be bundle)
 *
 * Updated regex to handle format correctly:
 * - Organization: alphanumeric with hyphens/underscores
 * - Slug: alphanumeric with underscores (no hyphens to avoid confusion with format)
 * - Version: alphanumeric with dots and underscores (after @, no hyphens)
 * - Format: alphanumeric with underscores (after final -)
 */
const SPECIAL_FORMAT_REGEX = /^([a-zA-Z0-9_-]+):([a-zA-Z0-9_]+)(?:@([a-zA-Z0-9._]+))?(?:-([a-zA-Z0-9_]+))?$/;

/**
 * Known bundle identifiers for organizations
 * These are treated as special cases
 */
const KNOWN_BUNDLES: Record<string, string[]> = {
  uniprot: ['swissprot', 'trembl'],
  genbank: ['refseq', 'genbank'],
  ensembl: ['vertebrates', 'plants', 'fungi', 'metazoa', 'protists', 'bacteria'],
};

/**
 * Parse a search query for special formats
 */
export function parseSearchQuery(query: string): ParsedQuery {
  const trimmedQuery = query.trim();

  // Try to match special format
  const match = trimmedQuery.match(SPECIAL_FORMAT_REGEX);

  if (!match) {
    // Not a special format, return as plain query
    return {
      original: query,
      isSpecialFormat: false,
      plainQuery: trimmedQuery,
    };
  }

  const [, organization, slug, version, format] = match;

  // Check if the slug is actually a known bundle
  const isBundle = KNOWN_BUNDLES[organization?.toLowerCase()]?.includes(slug?.toLowerCase());

  return {
    original: query,
    isSpecialFormat: true,
    organization,
    slug: isBundle ? undefined : slug,
    bundle: isBundle ? slug : undefined,
    version,
    format,
  };
}

/**
 * Convert parsed query to search filters
 */
export interface SearchFilters {
  organizations?: string[];
  sourceTypes?: string[];
  formats?: string[];
  version?: string;
  query?: string;
}

export function parsedQueryToFilters(parsed: ParsedQuery): SearchFilters {
  if (!parsed.isSpecialFormat) {
    return {
      query: parsed.plainQuery,
    };
  }

  const filters: SearchFilters = {};

  // Add organization filter
  if (parsed.organization) {
    filters.organizations = [parsed.organization];
  }

  // Use slug as the main search query if it's not a bundle
  if (parsed.slug) {
    filters.query = parsed.slug;
  }

  // Use bundle as the main search query if present
  if (parsed.bundle) {
    filters.query = parsed.bundle;
  }

  // Add format filter
  if (parsed.format) {
    filters.formats = [parsed.format];
  }

  // Add version filter
  if (parsed.version) {
    filters.version = parsed.version;
  }

  return filters;
}

/**
 * Format a parsed query for display
 */
export function formatParsedQuery(parsed: ParsedQuery): string {
  if (!parsed.isSpecialFormat) {
    return parsed.plainQuery || '';
  }

  const parts: string[] = [];

  if (parsed.organization) {
    parts.push(`Organization: ${parsed.organization}`);
  }

  if (parsed.slug) {
    parts.push(`Source: ${parsed.slug}`);
  }

  if (parsed.bundle) {
    parts.push(`Bundle: ${parsed.bundle}`);
  }

  if (parsed.version) {
    parts.push(`Version: ${parsed.version}`);
  }

  if (parsed.format) {
    parts.push(`Format: ${parsed.format}`);
  }

  return parts.join(' • ');
}

/**
 * Check if a query uses special format
 */
export function isSpecialFormat(query: string): boolean {
  return SPECIAL_FORMAT_REGEX.test(query.trim());
}

/**
 * Get examples of special format queries
 */
export function getSpecialFormatExamples(): string[] {
  return [
    'uniprot:P01308',
    'uniprot:swissprot',
    'uniprot:P01308@1.0',
    'uniprot:P01308-fasta',
    'uniprot:P01308@1.0-fasta',
    'genbank:refseq',
    'ensembl:vertebrates-gtf',
  ];
}
