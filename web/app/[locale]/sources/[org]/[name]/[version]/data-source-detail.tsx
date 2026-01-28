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
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import {
  Building2,
  Info,
  Database,
  ExternalLink,
} from 'lucide-react';
import type { DataSource, DataSourceVersion } from '@/lib/types/data-source';
import { CliCommands } from './cli-commands';
import { CitationsSection } from './citations-section';
import { DependenciesSection } from './dependencies-section';
import { MetadataSidebar } from '@/components/data-sources/metadata-sidebar';
import { SourceTypeContent } from '@/components/data-sources/source-type-content';
import { SourceTypeBadge } from '@/components/shared/source-type-badge';
import { SafeLink as Link } from '@/components/shared/safe-link';

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
  const [isMetadataOpen, setIsMetadataOpen] = React.useState(false);

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

  const handleMetadataOpenChange = React.useCallback((open: boolean) => {
    setIsMetadataOpen(open);
  }, []);

  // Extract clean protein/data source name without organism suffix
  const getCleanName = (name: string) => {
    // Remove organism in brackets pattern: [Organism name (Common name)]
    return name.replace(/\s*\[.*?\]\s*$/g, '').trim();
  };

  return (
    <div className="space-y-12">
      {/* Header - Full Width */}
      <div className="space-y-6">
        <div className="space-y-3">
          <div className="space-y-2">
            <h1 className="text-3xl font-bold tracking-tight">
              {getCleanName(dataSource.name)}
            </h1>

            {/* Organization and External Links */}
            <div className="flex items-center gap-4 flex-wrap">
              <Link
                href={`/${locale}/org/${dataSource.organization.slug}`}
                className="flex items-center gap-2 text-sm text-muted-foreground hover:text-primary transition-colors"
              >
                <Building2 className="h-4 w-4" />
                <span>{dataSource.organization.name}</span>
              </Link>

              {/* UniProt Website Link - only show for UniProt proteins */}
              {dataSource.organization.slug === 'uniprot' && dataSource.protein_metadata?.accession && (
                <a
                  href={`https://www.uniprot.org/uniprotkb/${dataSource.protein_metadata.accession}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-primary transition-colors"
                >
                  <ExternalLink className="h-4 w-4" />
                  <span>View on UniProt.org</span>
                </a>
              )}
            </div>

            {/* Source Type Badge */}
            <div>
              <SourceTypeBadge sourceType={dataSource.source_type} />
            </div>
          </div>

          {/* Version Selector */}
          <div className="w-64">
            <Select value={currentVersion.version} onValueChange={handleVersionChange}>
              <SelectTrigger>
                <SelectValue placeholder="Select version" />
              </SelectTrigger>
              <SelectContent>
                {dataSource.versions.map((version) => (
                  <SelectItem key={version.id} value={version.version}>
                    v{version.version}
                    {version.external_version && version.external_version !== 'unknown' && ` (${version.external_version})`}
                    {version.version === dataSource.latest_version && (
                      <span className="ml-2 text-xs text-primary">(latest)</span>
                    )}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Mobile Metadata Button - Only visible on mobile, below version selector */}
          <Dialog open={isMetadataOpen} onOpenChange={handleMetadataOpenChange} modal={true}>
            <DialogTrigger asChild>
              <Button variant="outline" size="sm" className="lg:hidden w-64">
                <Database className="h-4 w-4 mr-2" />
                <Info className="h-4 w-4 mr-2" />
                Data Source Info
              </Button>
            </DialogTrigger>
            <DialogContent className="sm:max-w-[425px] max-h-[85vh] overflow-y-auto">
              <DialogHeader>
                <DialogTitle>Data Source Info</DialogTitle>
                <DialogDescription>
                  View detailed metadata, statistics, and information about this data source.
                </DialogDescription>
              </DialogHeader>
              <div className="mt-4 w-full overflow-x-hidden">
                <MetadataSidebar
                  dataSource={dataSource}
                  currentVersion={currentVersion}
                  isInSheet={true}
                />
              </div>
            </DialogContent>
          </Dialog>
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

      {/* Two Column Layout */}
      <div className="grid grid-cols-1 lg:grid-cols-[1fr,320px] gap-12">
        {/* Main Content */}
        <div className="space-y-12">
          {/* Install with BDP CLI */}
          <div>
            <h2 className="mb-6 text-xl font-semibold">Install with BDP CLI</h2>
            <CliCommands
              org={dataSource.organization.slug}
              name={dataSource.slug}
              version={currentVersion.version}
              files={currentVersion.files}
            />
          </div>

          {/* Source-specific content sections */}
          <Separator />
          <SourceTypeContent dataSource={dataSource} currentVersion={currentVersion} />

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
        </div>

        {/* Sidebar */}
        <aside className="lg:block hidden">
          <MetadataSidebar dataSource={dataSource} currentVersion={currentVersion} />
        </aside>
      </div>
    </div>
  );
}
