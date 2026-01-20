'use client';

import * as React from 'react';
import { useRouter } from 'next/navigation';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Package,
  Calendar,
  Download,
  FileCode,
  HardDrive,
  ExternalLink,
  Building2,
  Dna,
} from 'lucide-react';
import type { DataSource, DataSourceVersion } from '@/lib/types/data-source';
import { CliCommands } from './cli-commands';
import { CitationsSection } from './citations-section';
import { DependenciesSection } from './dependencies-section';

interface DataSourceDetailProps {
  dataSource: DataSource;
  currentVersion: DataSourceVersion & { organization: string; name: string };
  locale: string;
}

export function DataSourceDetail({
  dataSource,
  currentVersion,
  locale,
}: DataSourceDetailProps) {
  const router = useRouter();

  const handleVersionChange = (newVersion: string) => {
    // Use Next.js router to navigate without full page refresh
    if (!dataSource.organization?.slug || !dataSource.slug) {
      console.error('Cannot change version: missing organization or data source slug');
      return;
    }
    router.push(
      `/${locale}/sources/${dataSource.organization.slug}/${dataSource.slug}/${newVersion}`
    );
  };

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i];
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
        <div className="flex items-start justify-between gap-4">
          <div className="flex-1 space-y-2">
            <div className="flex items-center gap-3">
              <h1 className="text-3xl font-bold tracking-tight">
                {dataSource.name}
              </h1>
              <Badge variant="outline" className="capitalize">
                {dataSource.source_type}
              </Badge>
            </div>

            {/* Organization */}
            <div className="flex items-center gap-2 text-muted-foreground">
              <Building2 className="h-4 w-4" />
              <span className="font-medium">{dataSource.organization.name}</span>
              {dataSource.organization.website && (
                <a
                  href={dataSource.organization.website}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center gap-1 text-primary hover:underline"
                >
                  <ExternalLink className="h-3 w-3" />
                </a>
              )}
            </div>

            {/* Description */}
            {dataSource.description && (
              <p className="text-base text-muted-foreground">
                {dataSource.description}
              </p>
            )}
          </div>

          {/* Version Selector */}
          <div className="w-48">
            <Select value={currentVersion.version} onValueChange={handleVersionChange}>
              <SelectTrigger>
                <SelectValue placeholder="Select version" />
              </SelectTrigger>
              <SelectContent>
                {dataSource.versions.map((version) => (
                  <SelectItem key={version.id} value={version.version}>
                    v{version.version}
                    {version.external_version && ` (${version.external_version})`}
                    {version.version === dataSource.latest_version && (
                      <span className="ml-2 text-xs text-primary">(latest)</span>
                    )}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>

        {/* Tags */}
        {dataSource.tags && dataSource.tags.length > 0 && (
          <div className="flex flex-wrap gap-2">
            {dataSource.tags.map((tag) => (
              <Badge key={tag} variant="secondary">
                {tag}
              </Badge>
            ))}
          </div>
        )}
      </div>

      <Separator />

      {/* Version Information */}
      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-4">
        <InfoCard
          icon={<Package className="h-5 w-5" />}
          label="Version"
          value={
            <div className="space-y-1">
              <div className="font-semibold">v{currentVersion.version}</div>
              {currentVersion.external_version && (
                <div className="text-xs text-muted-foreground">
                  {currentVersion.external_version}
                </div>
              )}
            </div>
          }
        />
        <InfoCard
          icon={<Calendar className="h-5 w-5" />}
          label="Release Date"
          value={
            currentVersion.release_date
              ? formatDate(currentVersion.release_date)
              : 'N/A'
          }
        />
        <InfoCard
          icon={<Download className="h-5 w-5" />}
          label="Downloads"
          value={currentVersion.download_count.toLocaleString()}
        />
        <InfoCard
          icon={<HardDrive className="h-5 w-5" />}
          label="Size"
          value={
            currentVersion.size_bytes ? formatBytes(currentVersion.size_bytes) : 'N/A'
          }
        />
      </div>

      {/* Protein Metadata */}
      {dataSource.protein_metadata && (
        <>
          <Separator />
          <div>
            <h2 className="mb-4 text-xl font-semibold">Protein Information</h2>
            <div className="grid gap-4 rounded-lg border bg-card p-6 md:grid-cols-2 lg:grid-cols-3">
              {dataSource.protein_metadata.accession && (
                <MetadataField
                  label="Accession"
                  value={dataSource.protein_metadata.accession}
                />
              )}
              {dataSource.protein_metadata.entry_name && (
                <MetadataField
                  label="Entry Name"
                  value={dataSource.protein_metadata.entry_name}
                />
              )}
              {dataSource.protein_metadata.gene_name && (
                <MetadataField
                  label="Gene"
                  value={dataSource.protein_metadata.gene_name}
                />
              )}
              {dataSource.organism && (
                <MetadataField
                  label="Organism"
                  value={
                    dataSource.organism.common_name ||
                    dataSource.organism.scientific_name
                  }
                />
              )}
              {dataSource.protein_metadata.sequence_length && (
                <MetadataField
                  label="Length"
                  value={`${dataSource.protein_metadata.sequence_length} aa`}
                />
              )}
              {dataSource.protein_metadata.mass_da && (
                <MetadataField
                  label="Mass"
                  value={`${dataSource.protein_metadata.mass_da.toLocaleString()} Da`}
                />
              )}
            </div>
          </div>
        </>
      )}

      {/* Available Formats & CLI Commands */}
      <Separator />
      <div>
        <h2 className="mb-4 text-xl font-semibold">Install with BDP CLI</h2>
        <CliCommands
          org={dataSource.organization.slug}
          name={dataSource.slug}
          version={currentVersion.version}
          files={currentVersion.files}
        />
      </div>

      {/* File Details */}
      <Separator />
      <div>
        <h2 className="mb-4 text-xl font-semibold">Available Files</h2>
        <div className="space-y-3">
          {currentVersion.files.map((file) => (
            <div
              key={file.id}
              className="flex items-center justify-between rounded-lg border bg-card p-4"
            >
              <div className="flex items-center gap-3">
                <FileCode className="h-5 w-5 text-muted-foreground" />
                <div>
                  <div className="font-medium uppercase">{file.format}</div>
                  <div className="text-sm text-muted-foreground">
                    {formatBytes(file.size_bytes)}
                    {file.compression && file.compression !== 'none' && (
                      <span> • {file.compression}</span>
                    )}
                  </div>
                </div>
              </div>
              <div className="text-xs text-muted-foreground font-mono">
                {file.checksum.substring(0, 16)}...
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Dependencies Section */}
      {currentVersion.has_dependencies && (
        <>
          <Separator />
          <DependenciesSection
            org={dataSource.organization.slug}
            name={dataSource.slug}
            version={currentVersion.version}
            dependencyCount={currentVersion.dependency_count}
          />
        </>
      )}

      {/* Citations */}
      {currentVersion.citations && currentVersion.citations.length > 0 && (
        <>
          <Separator />
          <CitationsSection citations={currentVersion.citations} />
        </>
      )}

      {/* Metadata */}
      <Separator />
      <div className="text-sm text-muted-foreground">
        <div>
          Created: {formatDate(dataSource.created_at)} • Last updated:{' '}
          {formatDate(dataSource.updated_at)}
        </div>
        <div className="mt-1">
          Total downloads across all versions: {dataSource.total_downloads.toLocaleString()}
        </div>
      </div>
    </div>
  );
}

function InfoCard({
  icon,
  label,
  value,
}: {
  icon: React.ReactNode;
  label: string;
  value: React.ReactNode;
}) {
  return (
    <div className="rounded-lg border bg-card p-4">
      <div className="flex items-center gap-2 text-muted-foreground mb-2">
        {icon}
        <span className="text-sm">{label}</span>
      </div>
      <div className="text-lg">{value}</div>
    </div>
  );
}

function MetadataField({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="text-sm text-muted-foreground">{label}</div>
      <div className="font-medium">{value}</div>
    </div>
  );
}
