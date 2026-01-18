'use client';

import * as React from 'react';
import { useRouter } from 'next/navigation';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import {
  Building2,
  ExternalLink,
  Package,
  Download,
  Layers,
  Calendar,
  ChevronLeft,
  ChevronRight,
} from 'lucide-react';
import { Link } from '@/i18n/navigation';
import type { Organization } from '@/lib/types/organization';
import type { DataSource } from '@/lib/types/data-source';

interface OrganizationDetailProps {
  organization: Organization;
  dataSources: DataSource[];
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

      {/* Data Sources */}
      <Separator />
      <div>
        <div className="mb-6 flex items-center justify-between">
          <div>
            <h2 className="text-2xl font-semibold">Data Sources</h2>
            <p className="text-sm text-muted-foreground mt-1">
              {pagination.total.toLocaleString()} total data sources
            </p>
          </div>
        </div>

        {/* Data Sources Grid */}
        {dataSources.length > 0 ? (
          <>
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              {dataSources.map((dataSource) => (
                <Link
                  key={dataSource.id}
                  href={`/sources/${dataSource.organization.slug}/${dataSource.slug}`}
                  className="group rounded-lg border bg-card p-4 transition-all hover:border-primary hover:shadow-md"
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

                    <div className="flex items-center gap-4 text-xs text-muted-foreground">
                      {dataSource.latest_version && (
                        <span>v{dataSource.latest_version}</span>
                      )}
                      <span className="flex items-center gap-1">
                        <Download className="h-3 w-3" />
                        {dataSource.total_downloads.toLocaleString()}
                      </span>
                    </div>

                    {dataSource.tags && dataSource.tags.length > 0 && (
                      <div className="flex flex-wrap gap-1">
                        {dataSource.tags.slice(0, 3).map((tag) => (
                          <Badge
                            key={tag}
                            variant="secondary"
                            className="text-xs"
                          >
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
              <div className="mt-8 flex items-center justify-between">
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
