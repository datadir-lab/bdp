'use client';

import * as React from 'react';
import { Sparkles } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import type { DataSource, DataSourceVersion } from '@/lib/types/data-source';

interface OrganismMetadataContentProps {
  dataSource: DataSource;
  currentVersion: DataSourceVersion & { organization: string; name: string };
}

export function OrganismMetadataContent({
  dataSource,
}: OrganismMetadataContentProps) {
  return (
    <div className="space-y-12">
      {/* Organism/Taxonomy-specific content */}
      {dataSource.organism && (
        <>
          <div>
            <div className="flex items-center gap-2 mb-6">
              <Sparkles className="h-5 w-5 text-muted-foreground" />
              <h2 className="text-xl font-semibold">Taxonomy Information</h2>
            </div>

            <div className="space-y-5">
              <div className="p-5 rounded-lg border bg-card">
                <div className="space-y-4">
                  <InfoField
                    label="Scientific Name"
                    value={<span className="italic">{dataSource.organism.scientific_name}</span>}
                  />
                  {dataSource.organism.common_name && (
                    <InfoField label="Common Name" value={dataSource.organism.common_name} />
                  )}
                  {dataSource.organism.rank && (
                    <InfoField
                      label="Rank"
                      value={<Badge variant="secondary">{dataSource.organism.rank}</Badge>}
                    />
                  )}
                  {dataSource.organism.ncbi_taxonomy_id && (
                    <InfoField
                      label="NCBI Taxonomy ID"
                      value={
                        <a
                          href={`https://www.ncbi.nlm.nih.gov/Taxonomy/Browser/wwwtax.cgi?id=${dataSource.organism.ncbi_taxonomy_id}`}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-primary hover:underline"
                        >
                          {dataSource.organism.ncbi_taxonomy_id}
                        </a>
                      }
                    />
                  )}
                </div>
              </div>
            </div>
          </div>
        </>
      )}
    </div>
  );
}

function InfoField({
  label,
  value,
}: {
  label: string;
  value: React.ReactNode;
}) {
  return (
    <div className="flex items-start justify-between gap-4">
      <span className="text-sm text-muted-foreground min-w-[120px]">{label}</span>
      <span className="font-medium text-sm text-right">{value}</span>
    </div>
  );
}
