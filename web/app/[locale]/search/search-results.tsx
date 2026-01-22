'use client';

import * as React from 'react';
import { useSearchParams, useRouter } from 'next/navigation';
import { useTranslations } from 'next-intl';
import { SearchBar } from '@/components/search/search-bar';
import { SearchFilters } from '@/components/search/search-filters';
import { SearchPagination } from '@/components/search/search-pagination';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { SourceTypeBadge } from '@/components/shared/source-type-badge';
import {
  SearchFilters as SearchFiltersType,
  SearchResult,
  SearchPagination as SearchPaginationType,
} from '@/lib/types/search';
import { searchFullText } from '@/lib/api/search';
import { Loader2, X } from 'lucide-react';
import { SafeLink as Link } from '@/components/shared/safe-link';

export function SearchResults() {
  const t = useTranslations('search');
  const searchParams = useSearchParams();
  const router = useRouter();
  const [isFiltersOpen, setIsFiltersOpen] = React.useState(false);
  const [results, setResults] = React.useState<SearchResult[]>([]);
  const [isLoading, setIsLoading] = React.useState(true);
  const [pagination, setPagination] = React.useState<SearchPaginationType | null>(null);
  const [error, setError] = React.useState<string | null>(null);

  // Parse filters from URL
  const [filters, setFilters] = React.useState<SearchFiltersType>(() => {
    const types = searchParams.get('types')?.split(',').filter(Boolean);
    const source_types = searchParams.get('source_types')?.split(',').filter(Boolean);
    const organizations = searchParams.get('organizations')?.split(',').filter(Boolean);
    const tags = searchParams.get('tags')?.split(',').filter(Boolean);
    const formats = searchParams.get('formats')?.split(',').filter(Boolean);
    const from = searchParams.get('from');
    const to = searchParams.get('to');

    return {
      types: types?.length ? types : undefined,
      source_types: source_types?.length ? source_types : undefined,
      organizations: organizations?.length ? organizations : undefined,
      tags: tags?.length ? tags : undefined,
      formats: formats?.length ? formats : undefined,
      dateRange:
        from || to
          ? {
              from: from ? new Date(from) : undefined,
              to: to ? new Date(to) : undefined,
            }
          : undefined,
    };
  });

  const query = searchParams.get('q') || '';
  const version = searchParams.get('version') || undefined;
  const currentPage = parseInt(searchParams.get('page') || '1', 10);

  // Fetch search results
  React.useEffect(() => {
    const fetchResults = async () => {
      if (!query) {
        setResults([]);
        setPagination(null);
        setIsLoading(false);
        return;
      }

      setIsLoading(true);
      setError(null);

      try {
        const data = await searchFullText({
          query,
          type_filter: filters.types,
          source_type_filter: filters.source_types,
          organism: filters.tags?.[0], // Using tags as organism filter for now
          format: filters.formats?.[0], // Use first format from filters
          version: version,
          page: currentPage,
          per_page: 20,
        });

        setResults(data.items);
        setPagination(data.pagination);
      } catch (err) {
        console.error('Search error:', err);
        setError('Failed to fetch search results');
        setResults([]);
        setPagination(null);
      } finally {
        setIsLoading(false);
      }
    };

    fetchResults();
  }, [query, filters, version, currentPage]);

  const handleFiltersChange = (newFilters: SearchFiltersType) => {
    setFilters(newFilters);
    // Update URL with new filters
    const params = new URLSearchParams();
    params.set('q', query);
    params.set('page', '1'); // Reset to page 1 when filters change
    if (newFilters.types?.length) params.set('types', newFilters.types.join(','));
    if (newFilters.source_types?.length) params.set('source_types', newFilters.source_types.join(','));
    if (newFilters.organizations?.length) params.set('organizations', newFilters.organizations.join(','));
    if (newFilters.tags?.length) params.set('tags', newFilters.tags.join(','));
    if (newFilters.formats?.length) params.set('formats', newFilters.formats.join(','));
    if (version) params.set('version', version);
    if (newFilters.dateRange?.from)
      params.set('from', newFilters.dateRange.from.toISOString());
    if (newFilters.dateRange?.to)
      params.set('to', newFilters.dateRange.to.toISOString());

    router.push(`/search?${params.toString()}`);
  };

  const handlePageChange = (page: number) => {
    const params = new URLSearchParams(searchParams.toString());
    params.set('page', page.toString());
    router.push(`/search?${params.toString()}`);
  };

  const clearFilter = (filterType: keyof SearchFiltersType, value?: string) => {
    const newFilters = { ...filters };

    if (filterType === 'types' && value) {
      newFilters.types = newFilters.types?.filter((t) => t !== value);
      if (newFilters.types?.length === 0) newFilters.types = undefined;
    } else if (filterType === 'source_types' && value) {
      newFilters.source_types = newFilters.source_types?.filter((t) => t !== value);
      if (newFilters.source_types?.length === 0) newFilters.source_types = undefined;
    } else if (filterType === 'organizations' && value) {
      newFilters.organizations = newFilters.organizations?.filter((o) => o !== value);
      if (newFilters.organizations?.length === 0) newFilters.organizations = undefined;
    } else if (filterType === 'tags' && value) {
      newFilters.tags = newFilters.tags?.filter((t) => t !== value);
      if (newFilters.tags?.length === 0) newFilters.tags = undefined;
    } else if (filterType === 'formats' && value) {
      newFilters.formats = newFilters.formats?.filter((f) => f !== value);
      if (newFilters.formats?.length === 0) newFilters.formats = undefined;
    } else if (filterType === 'dateRange') {
      newFilters.dateRange = undefined;
    }

    handleFiltersChange(newFilters);
  };

  const clearAllFilters = () => {
    handleFiltersChange({});
  };

  const getActiveFilters = () => {
    const active: Array<{ type: keyof SearchFiltersType; value: string; label: string }> =
      [];

    filters.types?.forEach((type) => {
      active.push({ type: 'types', value: type, label: type === 'datasource' ? 'Data Source' : 'Tool' });
    });

    filters.source_types?.forEach((sourceType) => {
      const label = sourceType.charAt(0).toUpperCase() + sourceType.slice(1);
      active.push({ type: 'source_types', value: sourceType, label });
    });

    filters.organizations?.forEach((org) => {
      active.push({ type: 'organizations', value: org, label: org.toUpperCase() });
    });

    filters.tags?.forEach((tag) => {
      const label = tag.charAt(0).toUpperCase() + tag.slice(1);
      active.push({ type: 'tags', value: tag, label });
    });

    filters.formats?.forEach((format) => {
      const label = `Format: ${format.toUpperCase()}`;
      active.push({ type: 'formats', value: format, label });
    });

    if (version) {
      active.push({
        type: 'tags', // Use tags as a proxy since version isn't in SearchFiltersType
        value: 'version',
        label: `Version: ${version}`,
      });
    }

    if (filters.dateRange) {
      active.push({
        type: 'dateRange',
        value: 'dateRange',
        label: 'Date Range',
      });
    }

    return active;
  };

  const activeFilters = getActiveFilters();

  // Helper to extract clean name without organism suffix
  const getCleanName = (name: string) => {
    return name.replace(/\s*\[.*?\]\s*$/g, '').trim();
  };

  return (
    <div className="space-y-6">
      {/* Search Bar */}
      <div className="space-y-4">
        <SearchBar
          variant="header"
          filters={filters}
          onFiltersOpen={() => setIsFiltersOpen(true)}
        />

        {/* Active Filters */}
        {activeFilters.length > 0 && (
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-sm text-muted-foreground">{t('activeFilters')}:</span>
            {activeFilters.map((filter, index) => (
              <Badge key={`${filter.type}-${filter.value}-${index}`} variant="secondary">
                <span className="capitalize">{filter.label}</span>
                <button
                  onClick={() => clearFilter(filter.type, filter.value)}
                  className="ml-1 rounded-full hover:bg-muted"
                >
                  <X className="h-3 w-3" />
                </button>
              </Badge>
            ))}
            <Button
              variant="ghost"
              size="sm"
              onClick={clearAllFilters}
              className="h-6 text-xs"
            >
              {t('clearAll')}
            </Button>
          </div>
        )}
      </div>

      <Separator />

      {/* Results Header */}
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">
          {isLoading ? (
            t('searching')
          ) : pagination && pagination.total > 0 ? (
            <>
              {pagination.total} {t('results')} {query && `for "${query}"`}
            </>
          ) : (
            t('noResults')
          )}
        </h1>
      </div>

      {/* Loading State */}
      {isLoading && (
        <div className="flex items-center justify-center py-12">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      )}

      {/* Error State */}
      {error && (
        <div className="rounded-lg border border-destructive bg-destructive/10 p-4 text-center">
          <p className="text-sm text-destructive">{error}</p>
        </div>
      )}

      {/* Results List */}
      {!isLoading && !error && results.length > 0 && (
        <>
          <div className="space-y-4">
            {results.map((result) => {
              // Check if data is missing
              const hasMissingData = !result.slug || (result.entry_type !== 'organization' && !result.organization_slug);

              // Skip rendering if critical data is missing
              if (hasMissingData) {
                console.warn('Skipping result with missing data:', result);
                return null;
              }

              // Construct proper href with version for data sources
              let href: string;
              if (result.entry_type === 'organization') {
                href = `/organizations/${result.slug}`;
              } else {
                // Include version in URL to avoid redirect issues
                const version = result.latest_version || 'latest';
                href = `/sources/${result.organization_slug}/${result.slug}/${version}`;
              }

              return (
                <Link
                  key={result.id}
                  href={href}
                  className="block rounded-lg border bg-card p-6 transition-all hover:border-primary hover:shadow-md"
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="flex-1 space-y-2">
                      <div className="flex items-center gap-2">
                        <h3 className="text-lg font-semibold">{getCleanName(result.name)}</h3>
                        <Badge variant="outline" className="capitalize">
                          {result.entry_type.replace('_', ' ')}
                        </Badge>
                        {result.latest_version && (
                          <Badge variant="secondary" className="text-xs">
                            v{result.latest_version}
                          </Badge>
                        )}
                      </div>
                      {result.description && (
                        <p className="text-sm text-muted-foreground">{result.description}</p>
                      )}
                      <div className="flex flex-wrap gap-2 pt-2">
                        {result.source_type && (
                          <SourceTypeBadge sourceType={result.source_type} />
                        )}
                        {result.available_formats.length > 0 && (
                          <Badge variant="secondary" className="text-xs">
                            {result.available_formats.join(', ')}
                          </Badge>
                        )}
                        {result.total_downloads > 0 && (
                          <Badge variant="secondary" className="text-xs">
                            {result.total_downloads} downloads
                          </Badge>
                        )}
                      </div>
                    </div>
                  </div>
                </Link>
              );
            })}
          </div>

          {/* Pagination */}
          {pagination && (
            <SearchPagination
              pagination={pagination}
              onPageChange={handlePageChange}
            />
          )}
        </>
      )}

      {/* Empty State */}
      {!isLoading && !error && results.length === 0 && query && (
        <div className="py-12 text-center">
          <p className="text-lg text-muted-foreground">
            {t('noResultsFor')} &quot;{query}&quot;
          </p>
          <p className="mt-2 text-sm text-muted-foreground">{t('tryDifferentQuery')}</p>
        </div>
      )}

      {/* Filters Dialog */}
      <SearchFilters
        open={isFiltersOpen}
        onOpenChange={setIsFiltersOpen}
        filters={filters}
        onFiltersChange={handleFiltersChange}
      />
    </div>
  );
}
