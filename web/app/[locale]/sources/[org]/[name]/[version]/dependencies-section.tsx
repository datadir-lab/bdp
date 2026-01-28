'use client';

import * as React from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { Search, ChevronLeft, ChevronRight, Loader2, Package } from 'lucide-react';
import { getDependencies } from '@/lib/api/data-sources';
import type { Dependency } from '@/lib/types/data-source';

interface DependenciesSectionProps {
  org: string;
  name: string;
  version: string;
  dependencyCount: number;
}

export function DependenciesSection({
  org,
  name,
  version,
  dependencyCount,
}: DependenciesSectionProps) {
  const [dependencies, setDependencies] = React.useState<Dependency[]>([]);
  const [isLoading, setIsLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);
  const [page, setPage] = React.useState(1);
  const [totalPages, setTotalPages] = React.useState(1);
  const [searchQuery, setSearchQuery] = React.useState('');
  const [debouncedSearch, setDebouncedSearch] = React.useState('');
  const [formatFilter, setFormatFilter] = React.useState<string>('all');
  const limit = 50;

  // Debounce search input
  React.useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedSearch(searchQuery);
      setPage(1); // Reset to first page on search
    }, 300);

    return () => clearTimeout(timer);
  }, [searchQuery]);

  // Fetch dependencies
  React.useEffect(() => {
    const fetchDependencies = async () => {
      setIsLoading(true);
      setError(null);

      try {
        const response = await getDependencies(org, name, version, {
          page,
          limit,
          search: debouncedSearch || undefined,
          format: formatFilter !== 'all' ? formatFilter : undefined,
        });

        setDependencies(response.dependencies);
        setTotalPages(response.pagination.pages);
      } catch (err) {
        console.error('Failed to fetch dependencies:', err);
        setError('Failed to load dependencies');
        setDependencies([]);
      } finally {
        setIsLoading(false);
      }
    };

    fetchDependencies();
  }, [org, name, version, page, debouncedSearch, formatFilter]);

  // Get unique types from first page for filter
  const availableTypes = React.useMemo(() => {
    const types = new Set(dependencies.map((dep) => dep.entry_type));
    return Array.from(types);
  }, [dependencies]);

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <div>
          <h2 className="text-xl font-semibold">Dependencies</h2>
          <p className="text-sm text-muted-foreground mt-1">
            {dependencyCount.toLocaleString()} total dependencies
          </p>
        </div>
      </div>

      {/* Search and Filter Controls */}
      <div className="flex flex-col sm:flex-row gap-3 mb-4">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search dependencies by name..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>

        <Select value={formatFilter} onValueChange={setFormatFilter}>
          <SelectTrigger className="w-full sm:w-40">
            <SelectValue placeholder="Type" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Types</SelectItem>
            {availableTypes.map((type) => (
              <SelectItem key={type} value={type}>
                {type.charAt(0).toUpperCase() + type.slice(1)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

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

      {/* Dependencies Table */}
      {!isLoading && !error && dependencies.length > 0 && (
        <>
          <div className="rounded-lg border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Data Source</TableHead>
                  <TableHead>Organization</TableHead>
                  <TableHead>Type</TableHead>
                  <TableHead>Version</TableHead>
                  <TableHead>Dependency Type</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {dependencies.map((dep) => (
                  <TableRow key={dep.id}>
                    <TableCell className="font-medium">
                      <div className="flex items-center gap-2">
                        <Package className="h-4 w-4 text-muted-foreground" />
                        <div>
                          <div>{dep.entry_name}</div>
                          <code className="text-xs text-muted-foreground">{dep.entry_slug}</code>
                        </div>
                      </div>
                    </TableCell>
                    <TableCell>
                      <Badge variant="outline">{dep.organization_slug}</Badge>
                    </TableCell>
                    <TableCell>
                      <Badge variant="secondary" className="capitalize">
                        {dep.entry_type}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <code className="text-xs">v{dep.required_version}</code>
                    </TableCell>
                    <TableCell>
                      <Badge variant="outline" className="capitalize">
                        {dep.dependency_type}
                      </Badge>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>

          {/* Pagination Controls */}
          {totalPages > 1 && (
            <div className="flex items-center justify-between mt-4">
              <div className="text-sm text-muted-foreground">
                Page {page} of {totalPages}
              </div>
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setPage((p) => Math.max(1, p - 1))}
                  disabled={page === 1}
                >
                  <ChevronLeft className="h-4 w-4 mr-1" />
                  Previous
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
                  disabled={page === totalPages}
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
      {!isLoading && !error && dependencies.length === 0 && (
        <div className="rounded-lg border bg-muted/50 p-12 text-center">
          <Package className="mx-auto h-12 w-12 text-muted-foreground mb-4" />
          <p className="text-lg text-muted-foreground">
            {debouncedSearch
              ? `No dependencies found matching "${debouncedSearch}"`
              : 'No dependencies found'}
          </p>
        </div>
      )}
    </div>
  );
}
