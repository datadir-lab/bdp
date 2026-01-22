'use client';

import * as React from 'react';
import {
  Building2,
  Scale,
  ExternalLink,
  Package,
  Calendar,
  HardDrive,
  Quote,
  Copy,
  Check,
  Download,
  Info,
} from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { DownloadGraph } from './download-graph';
import type { DataSource, DataSourceVersion } from '@/lib/types/data-source';

interface MetadataSidebarProps {
  dataSource: DataSource;
  currentVersion: DataSourceVersion & { organization: string; name: string };
  isInSheet?: boolean;
}

export function MetadataSidebar({ dataSource, currentVersion, isInSheet = false }: MetadataSidebarProps) {
  const [copiedBibtex, setCopiedBibtex] = React.useState(false);
  const [chartReady, setChartReady] = React.useState(!isInSheet);

  // Delay chart rendering in modal to ensure proper dimensions
  React.useEffect(() => {
    if (isInSheet) {
      setChartReady(false);
      // Double RAF to ensure layout is complete
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          setChartReady(true);
        });
      });
    }
  }, [isInSheet]);

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
      month: 'short',
      day: 'numeric',
    });
  };

  const generateBibtex = () => {
    const externalVersion = currentVersion.external_version && currentVersion.external_version !== 'unknown'
      ? currentVersion.external_version
      : currentVersion.version;
    const year = currentVersion.release_date
      ? new Date(currentVersion.release_date).getFullYear()
      : new Date(currentVersion.published_at).getFullYear();
    const slug = dataSource.slug.toLowerCase().replace(/[^a-z0-9]/g, '');

    // Construct original source URL based on organization
    let sourceUrl = '';
    if (dataSource.organization.slug === 'uniprot') {
      sourceUrl = `https://www.uniprot.org/uniprotkb/${dataSource.slug}`;
    } else {
      // Fallback to organization website if available
      sourceUrl = dataSource.organization.website || `https://bdp.dev/sources/${dataSource.organization.slug}/${dataSource.slug}/${currentVersion.version}`;
    }

    return `@misc{${slug}${year},
  title = {${dataSource.name}},
  author = {${dataSource.organization.name}},
  year = {${year}},
  version = {${externalVersion}},
  howpublished = {\\url{${sourceUrl}}},
  note = {Accessed via BDP: https://bdp.dev/sources/${dataSource.organization.slug}/${dataSource.slug}/${currentVersion.version}}
}`;
  };

  const copyBibtex = async () => {
    await navigator.clipboard.writeText(generateBibtex());
    setCopiedBibtex(true);
    setTimeout(() => setCopiedBibtex(false), 2000);
  };

  return (
    <div className={isInSheet ? "space-y-6 w-full" : "sticky top-20 space-y-6"}>
      {/* Download Graph */}
      <div className="rounded-lg border bg-card p-4 w-full overflow-hidden">
        {chartReady ? (
          <DownloadGraph
            downloadCount={currentVersion.download_count}
            totalDownloads={dataSource.total_downloads}
          />
        ) : (
          <div className="space-y-4">
            <div className="flex items-center gap-2">
              <Download className="h-4 w-4 text-muted-foreground" />
              <h3 className="font-semibold">Downloads</h3>
            </div>
            <div className="h-[180px] flex items-center justify-center">
              <div className="text-sm text-muted-foreground">Loading chart...</div>
            </div>
          </div>
        )}
      </div>

      {/* Version Info */}
      <div className="rounded-lg border bg-card p-4 space-y-4 w-full overflow-hidden">
        <h3 className="font-semibold">Version Info</h3>

        <div className="space-y-3 text-sm">
          <InfoRow
            icon={<Package className="h-4 w-4" />}
            label="Version"
            value={
              <div>
                <div className="font-mono break-all">v{currentVersion.version}</div>
                {currentVersion.external_version && currentVersion.external_version !== 'unknown' && (
                  <div className="text-xs text-muted-foreground break-all">
                    {currentVersion.external_version}
                  </div>
                )}
              </div>
            }
            tooltip="BDP internal version (top) tracks changes in the registry. External version (bottom) shows the original source version if available."
          />

          <InfoRow
            icon={<Calendar className="h-4 w-4" />}
            label="Published"
            value={new Date(currentVersion.published_at).toLocaleDateString('en-US', {
              year: 'numeric',
              month: 'short',
              day: 'numeric',
            })}
            tooltip="Published on BDP"
          />

          {currentVersion.size_bytes && (
            <InfoRow
              icon={<HardDrive className="h-4 w-4" />}
              label="Size"
              value={formatBytes(currentVersion.size_bytes)}
            />
          )}
        </div>
      </div>


      {/* Organization */}
      <div className="rounded-lg border bg-card p-4 space-y-4 w-full overflow-hidden">
        <div className="flex items-center gap-2">
          <Building2 className="h-4 w-4 text-muted-foreground" />
          <h3 className="font-semibold">Organization</h3>
        </div>

        <div className="space-y-2">
          <div className="font-medium break-words">{dataSource.organization.name}</div>
          {dataSource.organization.description && (
            <p className="text-sm text-muted-foreground break-words">
              {dataSource.organization.description}
            </p>
          )}
          {dataSource.organization.website && (
            <a
              href={dataSource.organization.website}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1 text-sm text-primary hover:underline break-all"
            >
              Visit website
              <ExternalLink className="h-3 w-3 shrink-0" />
            </a>
          )}
        </div>
      </div>

      {/* License */}
      <div className="rounded-lg border bg-card p-4 space-y-4 w-full overflow-hidden">
        <div className="flex items-center gap-2">
          <Scale className="h-4 w-4 text-muted-foreground" />
          <h3 className="font-semibold">License</h3>
        </div>

        <div className="text-sm space-y-3">
          {/* Placeholder - would come from backend */}
          <Badge variant="secondary">CC BY 4.0</Badge>
          <p className="text-muted-foreground">
            This data is freely available for research and commercial use.
          </p>
          <p className="text-xs text-muted-foreground italic">
            Not legal advice.
          </p>
        </div>
      </div>

      {/* Citation */}
      <div className="rounded-lg border bg-card p-4 space-y-4 w-full overflow-hidden">
        <div className="flex items-center gap-2">
          <Quote className="h-4 w-4 text-muted-foreground" />
          <h3 className="font-semibold">Cite this data</h3>
        </div>

        <div className="space-y-3">
          <div className="text-sm">
            <pre className="p-3 rounded-md bg-secondary text-xs overflow-x-auto max-w-full">
              <code className="break-all whitespace-pre-wrap">{generateBibtex()}</code>
            </pre>
          </div>

          <Button
            variant="outline"
            size="sm"
            className="w-full"
            onClick={copyBibtex}
          >
            {copiedBibtex ? (
              <>
                <Check className="h-3 w-3 mr-2" />
                Copied!
              </>
            ) : (
              <>
                <Copy className="h-3 w-3 mr-2" />
                Copy BibTeX
              </>
            )}
          </Button>

          <p className="text-xs text-muted-foreground">
            Or use <code className="px-1 py-0.5 rounded bg-secondary break-all">bdp cite {dataSource.organization.slug}:{dataSource.slug}@{currentVersion.version}</code>
          </p>
        </div>
      </div>

      {/* Organism metadata - only show for non-protein types */}
      {dataSource.organism && dataSource.source_type !== 'protein' && (
        <div className="rounded-lg border bg-card p-4 space-y-4 w-full overflow-hidden">
          <h3 className="font-semibold">Organism</h3>

          <div className="space-y-3 text-sm">
            <InfoRow
              label="Scientific Name"
              value={<span className="italic">{dataSource.organism.scientific_name}</span>}
            />
            {dataSource.organism.common_name && (
              <InfoRow label="Common Name" value={dataSource.organism.common_name} />
            )}
            {dataSource.organism.rank && (
              <InfoRow label="Rank" value={<span className="capitalize">{dataSource.organism.rank}</span>} />
            )}
            {dataSource.organism.ncbi_taxonomy_id && (
              <InfoRow
                label="Taxonomy ID"
                value={
                  <a
                    href={`https://www.ncbi.nlm.nih.gov/Taxonomy/Browser/wwwtax.cgi?id=${dataSource.organism.ncbi_taxonomy_id}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="inline-flex items-center gap-1 text-primary hover:underline break-all"
                  >
                    {dataSource.organism.ncbi_taxonomy_id}
                    <ExternalLink className="h-3 w-3 shrink-0" />
                  </a>
                }
              />
            )}
          </div>
        </div>
      )}

    </div>
  );
}

function InfoRow({
  icon,
  label,
  value,
  tooltip,
}: {
  icon?: React.ReactNode;
  label: string;
  value: React.ReactNode;
  tooltip?: string;
}) {
  return (
    <div className="flex justify-between items-start gap-4 min-w-0">
      <div className="flex items-center gap-2 text-muted-foreground shrink-0">
        {icon}
        <span>{label}</span>
        {tooltip && (
          <Tooltip>
            <TooltipTrigger asChild>
              <button type="button" className="inline-flex items-center">
                <Info className="h-3.5 w-3.5 cursor-help" />
                <span className="sr-only">More information</span>
              </button>
            </TooltipTrigger>
            <TooltipContent side="top" className="max-w-xs">
              <p>{tooltip}</p>
            </TooltipContent>
          </Tooltip>
        )}
      </div>
      <div className="text-right font-medium break-words min-w-0">{value}</div>
    </div>
  );
}
