/**
 * Tests for Query Parser
 *
 * Run with: npx jest query-parser.test.ts
 * Or just manually verify with: npx ts-node query-parser.test.ts
 */

import {
  parseSearchQuery,
  parsedQueryToFilters,
  formatParsedQuery,
  isSpecialFormat,
  getSpecialFormatExamples,
} from './query-parser';

// Test cases
const testCases = [
  {
    input: 'uniprot:P01308',
    expected: {
      isSpecialFormat: true,
      organization: 'uniprot',
      slug: 'P01308',
      version: undefined,
      format: undefined,
      bundle: undefined,
    },
    description: 'Basic org:slug format',
  },
  {
    input: 'uniprot:swissprot',
    expected: {
      isSpecialFormat: true,
      organization: 'uniprot',
      slug: undefined,
      bundle: 'swissprot',
      version: undefined,
      format: undefined,
    },
    description: 'Bundle format (org:bundle)',
  },
  {
    input: 'uniprot:P01308@1.0',
    expected: {
      isSpecialFormat: true,
      organization: 'uniprot',
      slug: 'P01308',
      version: '1.0',
      format: undefined,
      bundle: undefined,
    },
    description: 'Format with version (org:slug@version)',
  },
  {
    input: 'uniprot:P01308-fasta',
    expected: {
      isSpecialFormat: true,
      organization: 'uniprot',
      slug: 'P01308',
      version: undefined,
      format: 'fasta',
      bundle: undefined,
    },
    description: 'Format with format specifier (org:slug-format)',
  },
  {
    input: 'uniprot:P01308@1.0-fasta',
    expected: {
      isSpecialFormat: true,
      organization: 'uniprot',
      slug: 'P01308',
      version: '1.0',
      format: 'fasta',
      bundle: undefined,
    },
    description: 'Full format (org:slug@version-format)',
  },
  {
    input: 'genbank:refseq',
    expected: {
      isSpecialFormat: true,
      organization: 'genbank',
      slug: undefined,
      bundle: 'refseq',
      version: undefined,
      format: undefined,
    },
    description: 'GenBank bundle',
  },
  {
    input: 'ensembl:vertebrates-gtf',
    expected: {
      isSpecialFormat: true,
      organization: 'ensembl',
      slug: undefined,
      bundle: 'vertebrates',
      version: undefined,
      format: 'gtf',
    },
    description: 'Ensembl bundle with format',
  },
  {
    input: 'just a regular search',
    expected: {
      isSpecialFormat: false,
      plainQuery: 'just a regular search',
    },
    description: 'Plain query (no special format)',
  },
  {
    input: 'protein search',
    expected: {
      isSpecialFormat: false,
      plainQuery: 'protein search',
    },
    description: 'Multi-word plain query',
  },
];

// Run tests
console.log('🧪 Testing Query Parser\n');
console.log('='.repeat(80));

let passCount = 0;
let failCount = 0;

testCases.forEach((testCase, index) => {
  console.log(`\nTest ${index + 1}: ${testCase.description}`);
  console.log(`Input: "${testCase.input}"`);

  const result = parseSearchQuery(testCase.input);
  const isMatch = JSON.stringify({
    isSpecialFormat: result.isSpecialFormat,
    organization: result.organization,
    slug: result.slug,
    version: result.version,
    format: result.format,
    bundle: result.bundle,
    plainQuery: result.plainQuery,
  }) === JSON.stringify({
    ...testCase.expected,
    organization: testCase.expected.organization,
    slug: testCase.expected.slug,
    version: testCase.expected.version,
    format: testCase.expected.format,
    bundle: testCase.expected.bundle,
    plainQuery: testCase.expected.plainQuery,
  });

  if (isMatch) {
    console.log('✅ PASS');
    passCount++;
  } else {
    console.log('❌ FAIL');
    console.log('Expected:', testCase.expected);
    console.log('Got:', {
      isSpecialFormat: result.isSpecialFormat,
      organization: result.organization,
      slug: result.slug,
      version: result.version,
      format: result.format,
      bundle: result.bundle,
      plainQuery: result.plainQuery,
    });
    failCount++;
  }

  // Test parsedQueryToFilters
  const filters = parsedQueryToFilters(result);
  console.log('Filters:', JSON.stringify(filters, null, 2));

  // Test formatParsedQuery
  if (result.isSpecialFormat) {
    const formatted = formatParsedQuery(result);
    console.log('Formatted:', formatted);
  }

  // Test isSpecialFormat
  const isSpecial = isSpecialFormat(testCase.input);
  console.log('isSpecialFormat:', isSpecial);
});

console.log('\n' + '='.repeat(80));
console.log(`\n📊 Results: ${passCount} passed, ${failCount} failed`);

// Show examples
console.log('\n' + '='.repeat(80));
console.log('\n📚 Special Format Examples:');
const examples = getSpecialFormatExamples();
examples.forEach((example, index) => {
  console.log(`  ${index + 1}. ${example}`);
});

console.log('\n' + '='.repeat(80));
console.log('\n✨ Testing Complete!\n');

// Exit with appropriate code
if (failCount > 0) {
  process.exit(1);
} else {
  process.exit(0);
}
