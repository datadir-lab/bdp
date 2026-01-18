'use client';

import * as React from 'react';
import { useRouter } from 'next/navigation';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Package, Download, ChevronLeft, ChevronRight, Loader2 } from 'lucide-react';
import { Link } from '@/i18n/navigation';
import { listDataSources } from '@/lib/api/data-sources';
import type { DataSource } from '@/lib/types/data-source';

interface SourcesListProps {
  searchParams: {
    page?: string;
    org?: string;
    type?: string;
    sort?: string;
  };
}

export function SourcesList({ searchParams }: SourcesListProps) {
  const router = useRouter();
  const [dataSources, setDataSources] = React.useState<DataSource[]>([]);
  const [isLoading, setIsLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);
  const [pagination, setPagination] = React.useState({
    currentPage: 1,
    totalPages: 1,
    total: 0,
  });

  const currentPage = searchParams.page ? parseInt(searchParams.page) : 1;
  const orgFilter = searchParams.org;
  const typeFilter = searchParams.type || 'all';
  const sortBy = searchParams.sort || '-downloads';

  React.useEffect(() => {
    const fetchDataSources = async () => {
      setIsLoading(true);
      setError(null);

      try {
        const result = await listDataSources({
          page: currentPage,
          limit: 24,
          org: orgFilter,
          type: typeFilter !== 'all' ? typeFilter : undefined,
          sort: sortBy,
        });

        setDataSources(result.data);
        setPagination({
          currentPage,
          totalPages: result.pages,
          total: result.total,
        });
      } catch (err) {
        console.error('Failed to fetch data sources:', err);
        setError('Failed to load data sources');
        setDataSources([]);
      } finally {
        setIsLoading(false);
      }
    };

    fetchDataSources();
  }, [currentPage, orgFilter, typeFilter, sortBy]);

  const handleFilterChange = (key: string, value: string) => {
    const params = new URLSearchParams();
    if (value && value !== 'all') params.set(key, value);
    if (key !== 'type' && typeFilter !== 'all') params.set('type', typeFilter);
    if (key !== 'sort' && sortBy) params.set('sort', sortBy);
    if (key !== 'org' && orgFilter) params.set('org', orgFilter);

    const query = params.toString();
    router.push(`/sources${query ? `?${query}` : ''}`);
  };

  const handlePageChange = (page: number) => {
    const params = new URLSearchParams();
    params.set('page', page.toString());
    if (typeFilter !== 'all') params.set('type', typeFilter);
    if (sortBy) params.set('sort', sortBy);
    if (orgFilter) params.set('org', orgFilter);

    router.push(`/sources?${params.toString()}`);
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Data Sources</h1>
        <p className="text-muted-foreground mt-2">
          Browse all available biological data sources
        </p>
      </div>

      {/* Filters */}
      <div className="flex flex-wrap gap-3">
        <Select value={typeFilter} onValueChange={(val) => handleFilterChange('type', val)}>
          <SelectTrigger className="w-[180px]">
            <SelectValue placeholder="Type" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Types</SelectItem>
            <SelectItem value="protein">Protein</SelectItem>
            <SelectItem value="genome">Genome</SelectItem>
            <SelectItem value="annotation">Annotation</SelectItem>
            <SelectItem value="structure">Structure</SelectItem>
          </SelectContent>
        </Select>

        <Select value={sortBy} onValueChange={(val) => handleFilterChange('sort', val)}>
          <SelectTrigger className="w-[180px]">
            <SelectValue placeholder="Sort by" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="-downloads">Most Downloads</SelectItem>
            <SelectItem value="name">Name (A-Z)</SelectItem>
            <SelectItem value="-name">Name (Z-A)</SelectItem>
            <SelectItem value="-created_at">Newest First</SelectItem>
            <SelectItem value="created_at">Oldest First</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* Stats */}
      {!isLoading && !error && (
        <div className="text-sm text-muted-foreground">
          Showing {dataSources.length} of {pagination.total.toLocaleString()} data
          sources
        </div>
      )}

      {/* Loading State */}
      {isLoading && (
        <div className="flex items-center justify-center py-12">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      )}

      {/* Error State */}
      {error && !isLoading && (
        <div className="rounded-lg border border-destructive bg-destructive/10 p-4 text-center">
          <p className="text-sm text-destructive">{error}</p>
        </div>
      )}

      {/* Data Sources Grid */}
      {!isLoading && !error && dataSources.length > 0 && (
        <>
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {dataSources.map((dataSource) => (
              <Link
                key={dataSource.id}
                href={`/sources/${dataSource.organization.slug}/${dataSource.slug}`}
                className="group rounded-lg border bg-card p-5 transition-all hover:border-primary hover:shadow-md"
              >
                <div className="space-y-3">
                  <div className="flex items-start justify-between gap-2">
                    <h3 className="font-semibold group-hover:text-primary line-clamp-1">
                      {dataSource.name}
                    </h3>
                    <Badge variant="outline" className="shrink-0 capitalize">
                      {dataSource.source_type}
                    </Badge>
                  </div>

                  {dataSource.description && (
                    <p className="text-sm text-muted-foreground line-clamp-2">
                      {dataSource.description}
                    </p>
                  )}

                  <div className="flex items-center justify-between text-xs text-muted-foreground">
                    <span className="font-medium">
                      {dataSource.organization.name}
                    </span>
                    {dataSource.latest_version && (
                      <span>v{dataSource.latest_version}</span>
                    )}
                  </div>

                  <div className="flex items-center gap-1 text-xs text-muted-foreground">
                    <Download className="h-3 w-3" />
                    {dataSource.total_downloads.toLocaleString()} downloads
                  </div>

                  {dataSource.tags && dataSource.tags.length > 0 && (
                    <div className="flex flex-wrap gap-1">
                      {dataSource.tags.slice(0, 3).map((tag) => (
                        <Badge key={tag} variant="secondary" className="text-xs">
                          {tag}
                        </Badge>
                      ))}
                      {dataSource.tags.length > 3 && (
                        <Badge variant="secondary" className="text-xs">
                          +{dataSource.tags.length - 3}
                        </Badge>
                      )}
                    </div>
                  )}
                </div>
              </Link>
            ))}
          </div>

          {/* Pagination */}
          {pagination.totalPages > 1 && (
            <div className="flex items-center justify-between pt-4">
              <div className="text-sm text-muted-foreground">
                Page {pagination.currentPage} of {pagination.totalPages}
              </div>
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() =>
                    handlePageChange(Math.max(1, pagination.currentPage - 1))
                  }
                  disabled={pagination.currentPage === 1}
                >
                  <ChevronLeft className="h-4 w-4 mr-1" />
                  Previous
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() =>
                    handlePageChange(
                      Math.min(pagination.totalPages, pagination.currentPage + 1)
                    )
                  }
                  disabled={pagination.currentPage === pagination.totalPages}
                >
                  Next
                  <ChevronRight className="h-4 w-4 ml-1" />
                </Button>
              </div>
            </div>
          )}
        </>
      )}

      {/* Empty State */}
      {!isLoading && !error && dataSources.length === 0 && (
        <div className="rounded-lg border bg-muted/50 p-12 text-center">
          <Package className="mx-auto h-12 w-12 text-muted-foreground mb-4" />
          <p className="text-lg text-muted-foreground">No data sources found</p>
          <p className="text-sm text-muted-foreground mt-1">
            Try adjusting your filters
          </p>
        </div>
      )}
    </div>
  );
}
