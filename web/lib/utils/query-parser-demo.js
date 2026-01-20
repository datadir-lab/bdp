/**
 * Simple demonstration of the query parser
 * Run with: node web/lib/utils/query-parser-demo.js
 */

// Inline the parser logic for demonstration
// Updated regex: slug and version don't allow hyphens to avoid confusion with format
const SPECIAL_FORMAT_REGEX = /^([a-zA-Z0-9_-]+):([a-zA-Z0-9_]+)(?:@([a-zA-Z0-9._]+))?(?:-([a-zA-Z0-9_]+))?$/;

const KNOWN_BUNDLES = {
  uniprot: ['swissprot', 'trembl'],
  genbank: ['refseq', 'genbank'],
  ensembl: ['vertebrates', 'plants', 'fungi', 'metazoa', 'protists', 'bacteria'],
};

function parseSearchQuery(query) {
  const trimmedQuery = query.trim();
  const match = trimmedQuery.match(SPECIAL_FORMAT_REGEX);

  if (!match) {
    return {
      original: query,
      isSpecialFormat: false,
      plainQuery: trimmedQuery,
    };
  }

  const [, organization, slug, version, format] = match;
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

function formatParsedQuery(parsed) {
  if (!parsed.isSpecialFormat) {
    return parsed.plainQuery || '';
  }

  const parts = [];
  if (parsed.organization) parts.push(`Organization: ${parsed.organization}`);
  if (parsed.slug) parts.push(`Source: ${parsed.slug}`);
  if (parsed.bundle) parts.push(`Bundle: ${parsed.bundle}`);
  if (parsed.version) parts.push(`Version: ${parsed.version}`);
  if (parsed.format) parts.push(`Format: ${parsed.format}`);
  return parts.join(' • ');
}

// Test cases
const testCases = [
  'uniprot:P01308',
  'uniprot:swissprot',
  'uniprot:P01308@1.0',
  'uniprot:P01308-fasta',
  'uniprot:P01308@1.0-fasta',
  'genbank:refseq',
  'ensembl:vertebrates-gtf',
  'just a regular search',
  'protein search',
];

console.log('\n🧪 Query Parser Demonstration\n');
console.log('='.repeat(80));

testCases.forEach((testCase, index) => {
  console.log(`\n${index + 1}. Input: "${testCase}"`);
  const result = parseSearchQuery(testCase);

  console.log(`   Special Format: ${result.isSpecialFormat ? '✅' : '❌'}`);

  if (result.isSpecialFormat) {
    console.log(`   Organization: ${result.organization || 'N/A'}`);
    console.log(`   Slug: ${result.slug || 'N/A'}`);
    console.log(`   Bundle: ${result.bundle || 'N/A'}`);
    console.log(`   Version: ${result.version || 'N/A'}`);
    console.log(`   Format: ${result.format || 'N/A'}`);
    console.log(`   Formatted: ${formatParsedQuery(result)}`);
  } else {
    console.log(`   Plain Query: ${result.plainQuery}`);
  }
});

console.log('\n' + '='.repeat(80));
console.log('\n✨ Demonstration Complete!\n');
