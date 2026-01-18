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
import {
  Building2,
  ExternalLink,
  ChevronLeft,
  ChevronRight,
  Loader2,
} from 'lucide-react';
import { Link } from '@/i18n/navigation';
import { listOrganizations } from '@/lib/api/organizations';
import type { OrganizationListItem } from '@/lib/types/organization';

interface OrganizationsListProps {
  searchParams: {
    page?: string;
    sort?: string;
  };
}

export function OrganizationsList({ searchParams }: OrganizationsListProps) {
  const router = useRouter();
  const [organizations, setOrganizations] = React.useState<OrganizationListItem[]>(
    []
  );
  const [isLoading, setIsLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);
  const [pagination, setPagination] = React.useState({
    currentPage: 1,
    totalPages: 1,
    total: 0,
  });

  const currentPage = searchParams.page ? parseInt(searchParams.page) : 1;
  const sortBy = searchParams.sort || 'name';

  React.useEffect(() => {
    const fetchOrganizations = async () => {
      setIsLoading(true);
      setError(null);

      try {
        const result = await listOrganizations({
          page: currentPage,
          limit: 24,
          sort: sortBy,
        });

        setOrganizations(result.data);
        setPagination({
          currentPage,
          totalPages: result.pages,
          total: result.total,
        });
      } catch (err) {
        console.error('Failed to fetch organizations:', err);
        setError('Failed to load organizations');
        setOrganizations([]);
      } finally {
        setIsLoading(false);
      }
    };

    fetchOrganizations();
  }, [currentPage, sortBy]);

  const handleSortChange = (value: string) => {
    const params = new URLSearchParams();
    if (value) params.set('sort', value);

    const query = params.toString();
    router.push(`/organizations${query ? `?${query}` : ''}`);
  };

  const handlePageChange = (page: number) => {
    const params = new URLSearchParams();
    params.set('page', page.toString());
    if (sortBy) params.set('sort', sortBy);

    router.push(`/organizations?${params.toString()}`);
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Organizations</h1>
        <p className="text-muted-foreground mt-2">
          Browse organizations providing biological data
        </p>
      </div>

      {/* Sort */}
      <div className="flex items-center gap-3">
        <Select value={sortBy} onValueChange={handleSortChange}>
          <SelectTrigger className="w-[180px]">
            <SelectValue placeholder="Sort by" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="name">Name (A-Z)</SelectItem>
            <SelectItem value="-name">Name (Z-A)</SelectItem>
            <SelectItem value="-entry_count">Most Data Sources</SelectItem>
            <SelectItem value="-created_at">Newest First</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* Stats */}
      {!isLoading && !error && (
        <div className="text-sm text-muted-foreground">
          {pagination.total.toLocaleString()} organizations
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

      {/* Organizations Grid */}
      {!isLoading && !error && organizations.length > 0 && (
        <>
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {organizations.map((org) => (
              <Link
                key={org.id}
                href={`/organizations/${org.slug}`}
                className="group rounded-lg border bg-card p-5 transition-all hover:border-primary hover:shadow-md"
              >
                <div className="space-y-3">
                  <div className="flex items-start gap-3">
                    {org.logo_url && (
                      <img
                        src={org.logo_url}
                        alt={`${org.name} logo`}
                        className="h-12 w-12 rounded border object-contain p-1 shrink-0"
                      />
                    )}
                    <div className="flex-1 min-w-0">
                      <h3 className="font-semibold group-hover:text-primary line-clamp-1">
                        {org.name}
                      </h3>
                      {org.is_system && (
                        <Badge variant="secondary" className="text-xs mt-1">
                          Official
                        </Badge>
                      )}
                    </div>
                  </div>

                  {org.description && (
                    <p className="text-sm text-muted-foreground line-clamp-2">
                      {org.description}
                    </p>
                  )}

                  <div className="flex items-center justify-between text-xs text-muted-foreground pt-2">
                    <span className="flex items-center gap-1">
                      <Building2 className="h-3 w-3" />
                      {org.entry_count.toLocaleString()} data sources
                    </span>
                    <ExternalLink className="h-3 w-3 opacity-0 group-hover:opacity-100 transition-opacity" />
                  </div>
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
      {!isLoading && !error && organizations.length === 0 && (
        <div className="rounded-lg border bg-muted/50 p-12 text-center">
          <Building2 className="mx-auto h-12 w-12 text-muted-foreground mb-4" />
          <p className="text-lg text-muted-foreground">No organizations found</p>
        </div>
      )}
    </div>
  );
}
