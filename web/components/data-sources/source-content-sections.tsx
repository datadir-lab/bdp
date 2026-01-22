'use client';

import * as React from 'react';
import { Dna, Info, Sparkles, FileText } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import type { DataSource } from '@/lib/types/data-source';

interface SourceContentSectionsProps {
  dataSource: DataSource;
  currentVersion: any; // DataSourceVersion type from parent
}

export function SourceContentSections({ dataSource, currentVersion }: SourceContentSectionsProps) {
  const sourceType = dataSource.source_type;

  return (
    <div className="space-y-8">
      {/* Description */}
      {dataSource.description && (
        <div>
          <div className="flex items-center gap-2 mb-3">
            <Info className="h-5 w-5 text-muted-foreground" />
            <h2 className="text-xl font-semibold">Description</h2>
          </div>
          <p className="text-base leading-relaxed text-muted-foreground">
            {dataSource.description}
          </p>
        </div>
      )}

      {/* Protein-specific content */}
      {sourceType === 'protein' && dataSource.protein_metadata && (
        <>
          <Separator />
          <div>
            <div className="flex items-center gap-2 mb-4">
              <Dna className="h-5 w-5 text-muted-foreground" />
              <h2 className="text-xl font-semibold">Protein Information</h2>
            </div>

            <div className="grid gap-6 md:grid-cols-2">
              {/* Identifiers */}
              <div className="space-y-3">
                <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide">
                  Identifiers
                </h3>
                <div className="space-y-2">
                  {dataSource.protein_metadata.accession && (
                    <InfoField label="Accession" value={dataSource.protein_metadata.accession} />
                  )}
                  {dataSource.protein_metadata.entry_name && (
                    <InfoField label="Entry Name" value={dataSource.protein_metadata.entry_name} />
                  )}
                  {dataSource.protein_metadata.gene_name && (
                    <InfoField label="Gene Name" value={dataSource.protein_metadata.gene_name} />
                  )}
                </div>
              </div>

              {/* Physical Properties */}
              <div className="space-y-3">
                <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide">
                  Physical Properties
                </h3>
                <div className="space-y-2">
                  {dataSource.protein_metadata.sequence_length && (
                    <InfoField
                      label="Length"
                      value={`${dataSource.protein_metadata.sequence_length} amino acids`}
                    />
                  )}
                  {dataSource.protein_metadata.mass_da && (
                    <InfoField
                      label="Molecular Mass"
                      value={`${(dataSource.protein_metadata.mass_da / 1000).toFixed(2)} kDa`}
                    />
                  )}
                  {dataSource.protein_metadata.sequence_checksum && (
                    <InfoField
                      label="Checksum"
                      value={
                        <code className="text-xs px-2 py-1 rounded bg-secondary">
                          {dataSource.protein_metadata.sequence_checksum.substring(0, 16)}...
                        </code>
                      }
                    />
                  )}
                  {dataSource.protein_metadata.protein_existence && (
                    <InfoField
                      label="Existence"
                      value={`Level ${dataSource.protein_metadata.protein_existence}`}
                    />
                  )}
                </div>
              </div>
            </div>

            {/* Alternative Names */}
            {dataSource.protein_metadata.alternative_names && dataSource.protein_metadata.alternative_names.length > 0 && (
              <div className="mt-6">
                <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-2">
                  Alternative Names
                </h3>
                <div className="flex flex-wrap gap-2">
                  {dataSource.protein_metadata.alternative_names.map((name, idx) => (
                    <Badge key={idx} variant="outline">{name}</Badge>
                  ))}
                </div>
              </div>
            )}

            {/* EC Numbers */}
            {dataSource.protein_metadata.ec_numbers && dataSource.protein_metadata.ec_numbers.length > 0 && (
              <div className="mt-6">
                <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-2">
                  EC Numbers
                </h3>
                <div className="flex flex-wrap gap-2">
                  {dataSource.protein_metadata.ec_numbers.map((ec, idx) => (
                    <Badge key={idx} variant="secondary">{ec}</Badge>
                  ))}
                </div>
              </div>
            )}

            {/* Keywords */}
            {dataSource.protein_metadata.keywords && dataSource.protein_metadata.keywords.length > 0 && (
              <div className="mt-6">
                <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-2">
                  Keywords
                </h3>
                <div className="flex flex-wrap gap-2">
                  {dataSource.protein_metadata.keywords.map((keyword, idx) => (
                    <Badge key={idx} variant="secondary">{keyword}</Badge>
                  ))}
                </div>
              </div>
            )}

            {/* Organelle */}
            {dataSource.protein_metadata.organelle && (
              <div className="mt-6">
                <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-2">
                  Subcellular Location
                </h3>
                <Badge>{dataSource.protein_metadata.organelle}</Badge>
              </div>
            )}

            {/* Protein Comments */}
            {dataSource.protein_metadata.comments && dataSource.protein_metadata.comments.length > 0 && (
              <div className="mt-6">
                <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-3">
                  Protein Comments
                </h3>
                <div className="space-y-3">
                  {dataSource.protein_metadata.comments.map((comment, idx) => (
                    <div key={idx} className="p-3 rounded-lg border bg-muted/30">
                      <div className="text-xs font-semibold text-primary mb-1">{comment.topic}</div>
                      <div className="text-sm text-muted-foreground leading-relaxed">{comment.text}</div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Cross References */}
            {dataSource.protein_metadata.cross_references && dataSource.protein_metadata.cross_references.length > 0 && (
              <div className="mt-6">
                <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-3">
                  Database Cross-References
                </h3>
                <div className="grid grid-cols-2 md:grid-cols-3 gap-2">
                  {dataSource.protein_metadata.cross_references.slice(0, 30).map((xref, idx) => (
                    <div key={idx} className="flex items-center gap-2 text-sm">
                      <Badge variant="outline" className="font-mono text-xs">{xref.database}</Badge>
                      <span className="text-muted-foreground truncate">{xref.database_id}</span>
                    </div>
                  ))}
                </div>
                {dataSource.protein_metadata.cross_references.length > 30 && (
                  <p className="text-xs text-muted-foreground mt-2">
                    + {dataSource.protein_metadata.cross_references.length - 30} more references
                  </p>
                )}
              </div>
            )}

            {/* Protein Features */}
            {dataSource.protein_metadata.features && dataSource.protein_metadata.features.length > 0 && (
              <div className="mt-6">
                <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-3">
                  Protein Features
                </h3>
                <div className="space-y-2">
                  {dataSource.protein_metadata.features.slice(0, 20).map((feature, idx) => (
                    <div key={idx} className="flex items-start gap-3 text-sm">
                      <Badge variant="secondary" className="shrink-0">{feature.feature_type}</Badge>
                      <div className="flex-1">
                        {feature.description && (
                          <div className="text-muted-foreground">{feature.description}</div>
                        )}
                        {feature.start_pos && feature.end_pos && (
                          <div className="text-xs text-muted-foreground mt-1">
                            Position: {feature.start_pos}-{feature.end_pos}
                          </div>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
                {dataSource.protein_metadata.features.length > 20 && (
                  <p className="text-xs text-muted-foreground mt-2">
                    + {dataSource.protein_metadata.features.length - 20} more features
                  </p>
                )}
              </div>
            )}

            {/* Organism Info */}
            {dataSource.organism && (
              <div className="mt-6 p-4 rounded-lg border bg-muted/50">
                <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-3">
                  Organism
                </h3>
                <div className="space-y-2">
                  <div>
                    <div className="text-sm text-muted-foreground">Scientific Name</div>
                    <div className="font-medium italic">{dataSource.organism.scientific_name}</div>
                  </div>
                  {dataSource.organism.common_name && (
                    <div>
                      <div className="text-sm text-muted-foreground">Common Name</div>
                      <div className="font-medium">{dataSource.organism.common_name}</div>
                    </div>
                  )}
                  {dataSource.organism.rank && (
                    <div>
                      <div className="text-sm text-muted-foreground">Rank</div>
                      <div className="font-medium capitalize">{dataSource.organism.rank}</div>
                    </div>
                  )}
                  {dataSource.organism.ncbi_taxonomy_id && (
                    <div>
                      <div className="text-sm text-muted-foreground">NCBI Taxonomy ID</div>
                      <a
                        href={`https://www.ncbi.nlm.nih.gov/Taxonomy/Browser/wwwtax.cgi?id=${dataSource.organism.ncbi_taxonomy_id}`}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="font-medium text-primary hover:underline"
                      >
                        {dataSource.organism.ncbi_taxonomy_id}
                      </a>
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>
        </>
      )}

      {/* Organism/Taxonomy-specific content */}
      {sourceType === 'organism' && dataSource.organism && (
        <>
          <Separator />
          <div>
            <div className="flex items-center gap-2 mb-4">
              <Sparkles className="h-5 w-5 text-muted-foreground" />
              <h2 className="text-xl font-semibold">Taxonomy Information</h2>
            </div>

            <div className="space-y-4">
              <div className="p-4 rounded-lg border bg-card">
                <div className="space-y-3">
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

      {/* Generic metadata for other types */}
      {!['protein', 'organism'].includes(sourceType) && dataSource.external_id && (
        <>
          <Separator />
          <div>
            <div className="flex items-center gap-2 mb-4">
              <FileText className="h-5 w-5 text-muted-foreground" />
              <h2 className="text-xl font-semibold">Additional Information</h2>
            </div>

            <div className="p-4 rounded-lg border bg-card">
              <InfoField
                label="External ID"
                value={
                  <code className="text-sm px-2 py-1 rounded bg-secondary">
                    {dataSource.external_id}
                  </code>
                }
              />
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
