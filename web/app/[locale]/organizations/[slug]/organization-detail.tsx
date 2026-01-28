'use client';

import * as React from 'react';
import { useRouter } from 'next/navigation';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { Input } from '@/components/ui/input';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import {
  Building2,
  ExternalLink,
  Package,
  Download,
  Layers,
  Calendar,
  ChevronLeft,
  ChevronRight,
  Scale,
  BookOpen,
  GitBranch,
  FileText,
  Mail,
  Search,
} from 'lucide-react';
import { SafeLink as Link } from '@/components/shared/safe-link';
import type { Organization } from '@/lib/types/organization';
import type { DataSourceListItem } from '@/lib/types/data-source';

interface OrganizationDetailProps {
  organization: Organization;
  dataSources: DataSourceListItem[];
  pagination: {
    currentPage: number;
    totalPages: number;
    total: number;
  };
  locale: string;
}

export function OrganizationDetail({
  organization,
  dataSources,
  pagination,
  locale,
}: OrganizationDetailProps) {
  const router = useRouter();
  const [searchQuery, setSearchQuery] = React.useState('');

  const handlePageChange = (page: number) => {
    router.push(`/${locale}/organizations/${organization.slug}?page=${page}`);
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
    });
  };

  // Filter data sources based on search query
  const filteredDataSources = React.useMemo(() => {
    if (!searchQuery.trim()) return dataSources;

    const query = searchQuery.toLowerCase();
    return dataSources.filter((ds) => {
      return (
        ds.name?.toLowerCase().includes(query) ||
        ds.slug?.toLowerCase().includes(query) ||
        ds.external_id?.toLowerCase().includes(query) ||
        ds.organism_scientific_name?.toLowerCase().includes(query) ||
        ds.source_type?.toLowerCase().includes(query)
      );
    });
  }, [dataSources, searchQuery]);

  return (
    <div className="space-y-8">
      {/* Header */}
      <div className="space-y-4">
        <div className="flex items-start gap-6">
          {/* Logo */}
          {organization.logo_url && (
            <div className="shrink-0">
              <img
                src={organization.logo_url}
                alt={`${organization.name} logo`}
                className="h-24 w-24 rounded-lg border object-contain p-2"
              />
            </div>
          )}

          {/* Info */}
          <div className="flex-1 space-y-2">
            <div className="flex items-center gap-3">
              <h1 className="text-3xl font-bold tracking-tight">
                {organization.name}
              </h1>
              {organization.is_system && (
                <Badge variant="secondary">Official</Badge>
              )}
            </div>

            {organization.website && (
              <a
                href={organization.website}
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex items-center gap-1 text-sm text-primary hover:underline"
              >
                {organization.website}
                <ExternalLink className="h-3 w-3" />
              </a>
            )}

            {organization.description && (
              <p className="text-base text-muted-foreground">
                {organization.description}
              </p>
            )}
          </div>
        </div>
      </div>

      {/* Statistics */}
      {organization.statistics && (
        <>
          <Separator />
          <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-4">
            <StatCard
              icon={<Package className="h-5 w-5" />}
              label="Data Sources"
              value={organization.statistics.data_sources.toLocaleString()}
            />
            <StatCard
              icon={<Layers className="h-5 w-5" />}
              label="Total Versions"
              value={organization.statistics.total_versions.toLocaleString()}
            />
            <StatCard
              icon={<Download className="h-5 w-5" />}
              label="Total Downloads"
              value={organization.statistics.total_downloads.toLocaleString()}
            />
            <StatCard
              icon={<Calendar className="h-5 w-5" />}
              label="Member Since"
              value={formatDate(organization.created_at).split(',')[1].trim()}
            />
          </div>
        </>
      )}

      {/* Licensing, Citation, and Additional Information */}
      {(organization.license ||
        organization.citation ||
        organization.version_strategy ||
        organization.data_source_url ||
        organization.documentation_url ||
        organization.contact_email) && (
        <>
          <Separator />
          <div className="space-y-6">
            <h2 className="text-2xl font-semibold">Organization Information</h2>

            <div className="grid gap-6 md:grid-cols-2">
              {/* Licensing */}
              {organization.license && (
                <InfoCard
                  icon={<Scale className="h-5 w-5" />}
                  title="License"
                  content={organization.license}
                  link={organization.license_url}
                />
              )}

              {/* Citation */}
              {organization.citation && (
                <InfoCard
                  icon={<FileText className="h-5 w-5" />}
                  title="How to Cite"
                  content={organization.citation}
                  link={organization.citation_url}
                />
              )}

              {/* Version Strategy */}
              {organization.version_strategy && (
                <div className="rounded-lg border bg-card p-4 md:col-span-2">
                  <div className="flex items-center gap-2 mb-3">
                    <div className="text-primary">
                      <GitBranch className="h-5 w-5" />
                    </div>
                    <h3 className="font-semibold">Version Strategy</h3>
                  </div>
                  <div className="space-y-3 text-sm">
                    <div>
                      <p className="font-medium text-foreground mb-1">
                        {organization.name} Versioning
                      </p>
                      <p className="text-muted-foreground">
                        {organization.version_description ||
                          organization.version_strategy}
                      </p>
                    </div>
                    <Separator />
                    <div>
                      <p className="font-medium text-foreground mb-1">
                        BDP Internal Versioning
                      </p>
                      <p className="text-muted-foreground">
                        BDP tracks each data source ingestion with an internal
                        semantic version (e.g., 1.0, 1.1) mapped to the
                        external version. Each ingestion job creates a new
                        immutable version, preserving historical data for
                        reproducibility.
                      </p>
                    </div>
                  </div>
                </div>
              )}

              {/* Documentation */}
              {organization.documentation_url && (
                <InfoCard
                  icon={<BookOpen className="h-5 w-5" />}
                  title="Documentation"
                  content="View documentation"
                  link={organization.documentation_url}
                />
              )}

              {/* Data Source */}
              {organization.data_source_url && (
                <InfoCard
                  icon={<ExternalLink className="h-5 w-5" />}
                  title="Original Data Source"
                  content="Visit data source"
                  link={organization.data_source_url}
                />
              )}

              {/* Contact */}
              {organization.contact_email && (
                <InfoCard
                  icon={<Mail className="h-5 w-5" />}
                  title="Contact"
                  content={organization.contact_email}
                  link={`mailto:${organization.contact_email}`}
                />
              )}
            </div>
          </div>
        </>
      )}

      {/* Data Sources */}
      <Separator />
      <div>
        <div className="mb-6 space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-2xl font-semibold">Data Sources</h2>
              <p className="text-sm text-muted-foreground mt-1">
                {pagination.total.toLocaleString()} total data sources
              </p>
            </div>
          </div>

          {/* Search */}
          {dataSources.length > 0 && (
            <div className="relative max-w-md">
              <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                type="text"
                placeholder="Search by name, slug, type, or organism..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="pl-9"
              />
            </div>
          )}
        </div>

        {/* Data Sources Table */}
        {dataSources.length > 0 ? (
          <>
            {filteredDataSources.length > 0 ? (
              <div className="rounded-lg border">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Name</TableHead>
                      <TableHead>Type</TableHead>
                      <TableHead>Organism</TableHead>
                      <TableHead>Version</TableHead>
                      <TableHead className="text-right">Downloads</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {filteredDataSources.map((dataSource) => {
                      // Skip data sources with missing required fields
                      if (!dataSource.slug || !dataSource.organization_slug) {
                        console.warn(
                          'Skipping data source with missing slug:',
                          dataSource
                        );
                        return null;
                      }

                      const version = dataSource.latest_version || 'latest';
                      const href = `/sources/${dataSource.organization_slug}/${dataSource.slug}/${version}`;

                      return (
                        <TableRow
                          key={dataSource.id}
                          className="cursor-pointer"
                          onClick={() => router.push(href)}
                        >
                          <TableCell className="font-medium">
                            <div className="space-y-1">
                              <div className="flex items-center gap-2">
                                <span className="hover:text-primary">
                                  {dataSource.name}
                                </span>
                              </div>
                              {dataSource.external_id && (
                                <div className="text-xs text-muted-foreground">
                                  {dataSource.external_id}
                                </div>
                              )}
                            </div>
                          </TableCell>
                          <TableCell>
                            <Badge variant="outline" className="capitalize">
                              {dataSource.source_type}
                            </Badge>
                          </TableCell>
                          <TableCell className="max-w-[200px]">
                            <span className="text-sm text-muted-foreground line-clamp-1">
                              {dataSource.organism_scientific_name || '-'}
                            </span>
                          </TableCell>
                          <TableCell>
                            {dataSource.latest_version && (
                              <span className="text-sm">
                                v{dataSource.latest_version}
                              </span>
                            )}
                          </TableCell>
                          <TableCell className="text-right">
                            <div className="flex items-center justify-end gap-1 text-sm text-muted-foreground">
                              <Download className="h-3 w-3" />
                              {dataSource.total_downloads.toLocaleString()}
                            </div>
                          </TableCell>
                        </TableRow>
                      );
                    })}
                  </TableBody>
                </Table>
              </div>
            ) : (
              <div className="rounded-lg border bg-muted/50 p-12 text-center">
                <Search className="mx-auto h-12 w-12 text-muted-foreground mb-4" />
                <p className="text-lg text-muted-foreground mb-2">
                  No data sources match your search
                </p>
                <p className="text-sm text-muted-foreground">
                  Try a different search term or clear the filter
                </p>
              </div>
            )}

            {/* Pagination */}
            {pagination.totalPages > 1 && (
              <div className="mt-6 flex items-center justify-between">
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
        ) : (
          <div className="rounded-lg border bg-muted/50 p-12 text-center">
            <Package className="mx-auto h-12 w-12 text-muted-foreground mb-4" />
            <p className="text-lg text-muted-foreground">
              No data sources found for this organization
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

function StatCard({
  icon,
  label,
  value,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
}) {
  return (
    <div className="rounded-lg border bg-card p-4">
      <div className="flex items-center gap-2 text-muted-foreground mb-2">
        {icon}
        <span className="text-sm">{label}</span>
      </div>
      <div className="text-2xl font-bold">{value}</div>
    </div>
  );
}

function InfoCard({
  icon,
  title,
  content,
  link,
}: {
  icon: React.ReactNode;
  title: string;
  content: string;
  link?: string;
}) {
  return (
    <div className="rounded-lg border bg-card p-4">
      <div className="flex items-center gap-2 mb-3">
        <div className="text-primary">{icon}</div>
        <h3 className="font-semibold">{title}</h3>
      </div>
      {link ? (
        <a
          href={link}
          target="_blank"
          rel="noopener noreferrer"
          className="inline-flex items-center gap-1 text-sm text-primary hover:underline"
        >
          {content}
          <ExternalLink className="h-3 w-3" />
        </a>
      ) : (
        <p className="text-sm text-muted-foreground">{content}</p>
      )}
    </div>
  );
}
