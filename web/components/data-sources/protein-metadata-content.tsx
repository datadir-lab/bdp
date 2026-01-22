'use client';

import * as React from 'react';
import { Info, Loader2, Copy, Check, ArrowRight } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import type { DataSource, DataSourceVersion } from '@/lib/types/data-source';
import { getProteinMetadata } from '@/lib/api/data-sources';

interface ProteinMetadataContentProps {
  dataSource: DataSource;
  currentVersion: DataSourceVersion & { organization: string; name: string };
}

export function ProteinMetadataContent({
  dataSource,
  currentVersion,
}: ProteinMetadataContentProps) {
  const [metadata, setMetadata] = React.useState<{
    comments: Array<{ topic: string; text: string }>;
    features: Array<{
      feature_type: string;
      description?: string;
      start_pos?: number;
      end_pos?: number;
    }>;
    cross_references: Array<{
      database: string;
      database_id: string;
      metadata?: any;
    }>;
  } | null>(null);
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);
  const [copiedChecksum, setCopiedChecksum] = React.useState(false);

  const copyChecksum = async () => {
    if (dataSource.protein_metadata?.sequence_checksum) {
      await navigator.clipboard.writeText(dataSource.protein_metadata.sequence_checksum);
      setCopiedChecksum(true);
      setTimeout(() => setCopiedChecksum(false), 2000);
    }
  };

  // Tooltips for different comment topics
  const getCommentTooltip = (topic: string): string => {
    const tooltips: Record<string, string> = {
      'FUNCTION': 'General function of the protein and its role in biological processes.',
      'CATALYTIC ACTIVITY': 'The biochemical reaction(s) that this protein catalyzes, including substrates and products.',
      'SUBUNIT': 'Quaternary structure and protein-protein interactions. Describes oligomeric state and subunit composition.',
      'TISSUE SPECIFICITY': 'Expression pattern across different tissues, organs, or developmental stages.',
      'DISEASE': 'Association with human diseases, including mutations and their clinical effects.',
      'PTM': 'Post-translational modifications such as phosphorylation, acetylation, or glycosylation.',
      'SUBCELLULAR LOCATION': 'Where the protein is located within the cell (nucleus, membrane, cytoplasm, etc.).',
      'SIMILARITY': 'Sequence and structural similarities to other proteins or protein families.',
      'COFACTOR': 'Non-protein chemical compounds required for the protein\'s biological activity.',
      'PATHWAY': 'Metabolic or signaling pathways in which this protein participates.',
      'DOMAIN': 'Structural and functional domains within the protein sequence.',
      'INDUCTION': 'Conditions or signals that increase expression of this protein.',
      'MISCELLANEOUS': 'Additional information that doesn\'t fit other categories.',
    };
    return tooltips[topic.toUpperCase()] || 'Annotated information about this protein.';
  };

  // Clean feature description by removing metadata tags
  const cleanFeatureDescription = (description: string | undefined): string | null => {
    if (!description) return null;

    // Remove /FTId and other metadata tags that start with /
    const cleaned = description
      .split('\n')
      .filter(line => !line.trim().startsWith('/'))
      .map(line => line.trim())
      .filter(line => line.length > 0)
      .join(' ')
      .replace(/\{[^}]*\}/g, '') // Remove evidence codes in curly braces
      .replace(/\s+/g, ' ') // Normalize whitespace
      .trim();

    return cleaned.length > 0 ? cleaned : null;
  };

  React.useEffect(() => {
    async function fetchMetadata() {
      try {
        setLoading(true);
        setError(null);
        const data = await getProteinMetadata(
          dataSource.organization.slug,
          dataSource.slug,
          currentVersion.version
        );
        setMetadata(data);
      } catch (err) {
        console.error('Error fetching protein metadata:', err);
        setError('Failed to load protein metadata');
      } finally {
        setLoading(false);
      }
    }

    fetchMetadata();
  }, [dataSource.organization.slug, dataSource.slug, currentVersion.version]);

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-4">
        <p className="text-sm text-destructive">{error}</p>
      </div>
    );
  }

  if (!metadata) {
    return null;
  }

  return (
    <div className="space-y-12">
        {/* Protein-specific content */}
        {dataSource.protein_metadata && (
          <>
            <div>
            <h2 className="text-xl font-semibold mb-6">Protein Information</h2>

            <div className="grid gap-8 md:grid-cols-2">
              {/* Identifiers */}
              <div className="space-y-4">
                <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide">
                  Identifiers
                </h3>
                <div className="space-y-3">
                  {dataSource.protein_metadata.accession && (
                    <InfoField
                      label="Accession"
                      value={dataSource.protein_metadata.accession}
                      tooltip="Unique identifier assigned to this protein entry in the database. This is the primary identifier used to reference this protein."
                    />
                  )}
                  {dataSource.protein_metadata.entry_name && (
                    <InfoField
                      label="Entry Name"
                      value={dataSource.protein_metadata.entry_name}
                      tooltip="Mnemonic name for the protein entry, typically combining the protein name and organism. More human-readable than the accession number."
                    />
                  )}
                  {dataSource.protein_metadata.gene_name && (
                    <InfoField
                      label="Gene Name"
                      value={dataSource.protein_metadata.gene_name}
                      tooltip="Name of the gene that encodes this protein. Gene names follow standard nomenclature for the organism."
                    />
                  )}
                </div>
              </div>

              {/* Physical Properties */}
              <div className="space-y-4">
                <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide">
                  Physical Properties
                </h3>
                <div className="space-y-3">
                  {dataSource.protein_metadata.sequence_length && (
                    <InfoField
                      label="Length"
                      value={`${dataSource.protein_metadata.sequence_length} amino acids`}
                      tooltip="The number of amino acids in the protein sequence. Longer sequences generally indicate larger, more complex proteins."
                    />
                  )}
                  {dataSource.protein_metadata.mass_da && (
                    <InfoField
                      label="Molecular Mass"
                      value={`${(dataSource.protein_metadata.mass_da / 1000).toFixed(2)} kDa`}
                      tooltip="The molecular weight of the protein in kilodaltons (kDa). Calculated from the amino acid sequence without post-translational modifications."
                    />
                  )}
                  {dataSource.protein_metadata.sequence_checksum && (
                    <InfoField
                      label="Checksum"
                      value={
                        <div className="flex items-center gap-2">
                          <code className="text-xs px-2 py-1 rounded bg-secondary">
                            {dataSource.protein_metadata.sequence_checksum.substring(0, 16)}...
                          </code>
                          <button
                            onClick={copyChecksum}
                            className="text-muted-foreground hover:text-foreground transition-colors"
                            aria-label="Copy checksum"
                          >
                            {copiedChecksum ? (
                              <Check className="h-3.5 w-3.5 text-green-600" />
                            ) : (
                              <Copy className="h-3.5 w-3.5" />
                            )}
                          </button>
                        </div>
                      }
                      tooltip="CRC64 checksum of the protein sequence. Used to verify sequence integrity and detect any changes or errors in the sequence data."
                    />
                  )}
                  {dataSource.protein_metadata.protein_existence && (
                    <InfoField
                      label="Existence"
                      value={`Level ${dataSource.protein_metadata.protein_existence}`}
                      tooltip="Evidence level for protein existence: 1 = Evidence at protein level, 2 = Evidence at transcript level, 3 = Inferred from homology, 4 = Predicted, 5 = Uncertain."
                    />
                  )}
                </div>
              </div>
            </div>

            {/* Organism Info */}
            {dataSource.organism && (
              <div className="mt-8">
                {(() => {
                  console.log('Organism data:', {
                    taxonomy_organization_slug: dataSource.organism.taxonomy_organization_slug,
                    taxonomy_slug: dataSource.organism.taxonomy_slug,
                    taxonomy_version: dataSource.organism.taxonomy_version,
                    ncbi_taxonomy_id: dataSource.organism.ncbi_taxonomy_id,
                    full_organism: dataSource.organism
                  });
                  return null;
                })()}
                {dataSource.organism.taxonomy_organization_slug && dataSource.organism.taxonomy_slug && dataSource.organism.taxonomy_version ? (
                  <a
                    href={`/sources/${dataSource.organism.taxonomy_organization_slug}/${dataSource.organism.taxonomy_slug}/${dataSource.organism.taxonomy_version}`}
                    className="group p-5 rounded-lg border bg-muted/50 block hover:border-primary/50 hover:bg-muted transition-all"
                    style={{ textDecoration: 'none' }}
                  >
                    <div className="flex items-center justify-between mb-4">
                      <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide">
                        Organism
                      </h3>
                      <ArrowRight className="h-4 w-4 text-muted-foreground group-hover:text-primary group-hover:translate-x-0.5 transition-all" />
                    </div>
                    <div className="space-y-3">
                      <div>
                        <div className="text-sm text-muted-foreground">Scientific Name</div>
                        <div className="font-medium italic text-foreground">{dataSource.organism.scientific_name}</div>
                      </div>
                      {dataSource.organism.common_name && (
                        <div>
                          <div className="text-sm text-muted-foreground">Common Name</div>
                          <div className="font-medium text-foreground">{dataSource.organism.common_name}</div>
                        </div>
                      )}
                      {dataSource.organism.rank && (
                        <div>
                          <div className="text-sm text-muted-foreground">Rank</div>
                          <div className="font-medium capitalize text-foreground">{dataSource.organism.rank}</div>
                        </div>
                      )}
                      <div>
                        <div className="text-sm text-muted-foreground">Taxonomy ID</div>
                        <div className="font-medium font-mono text-foreground">{dataSource.organism.ncbi_taxonomy_id}</div>
                      </div>
                    </div>
                  </a>
                ) : (
                  <div className="p-5 rounded-lg border bg-muted/50">
                    <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-4">
                      Organism
                    </h3>
                    <div className="space-y-3">
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
                      <div className="mt-4 p-3 rounded bg-yellow-500/10 border border-yellow-500/20">
                        <div className="text-xs text-yellow-700 dark:text-yellow-400">
                          <div className="font-semibold mb-1">Debug: Taxonomy link unavailable</div>
                          <div className="space-y-1 font-mono">
                            <div>org: {dataSource.organism.taxonomy_organization_slug || 'null'}</div>
                            <div>slug: {dataSource.organism.taxonomy_slug || 'null'}</div>
                            <div>version: {dataSource.organism.taxonomy_version || 'null'}</div>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                )}
              </div>
            )}

            {/* Alternative Names */}
            {dataSource.protein_metadata.alternative_names && dataSource.protein_metadata.alternative_names.length > 0 && (
              <div className="mt-8">
                <SectionHeader
                  label="Alternative Names"
                  tooltip="Other names by which this protein is known. Includes synonyms, short names, and full names from different nomenclature systems."
                />
                <div className="flex flex-wrap gap-2">
                  {dataSource.protein_metadata.alternative_names.map((name, idx) => (
                    <Badge key={idx} variant="outline">{name}</Badge>
                  ))}
                </div>
              </div>
            )}

            {/* EC Numbers */}
            {dataSource.protein_metadata.ec_numbers && dataSource.protein_metadata.ec_numbers.length > 0 && (
              <div className="mt-8">
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
              <div className="mt-8">
                <SectionHeader
                  label="Keywords"
                  tooltip="Controlled vocabulary terms describing the protein's function, cellular location, domain structure, and biological processes. Used for categorization and searching."
                />
                <div className="flex flex-wrap gap-2">
                  {dataSource.protein_metadata.keywords.map((keyword, idx) => (
                    <Badge key={idx} variant="secondary">{keyword}</Badge>
                  ))}
                </div>
              </div>
            )}

            {/* Organelle */}
            {dataSource.protein_metadata.organelle && (
              <div className="mt-8">
                <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-2">
                  Subcellular Location
                </h3>
                <Badge>{dataSource.protein_metadata.organelle}</Badge>
              </div>
            )}

            {/* Protein Comments */}
            {metadata.comments && metadata.comments.length > 0 && (
              <div className="mt-8">
                <SectionHeader
                  label="Protein Annotations"
                  tooltip="Curated annotations about the protein's function, catalytic activity, subunit structure, tissue specificity, disease associations, and other biological characteristics."
                />
                <div className="space-y-5 mt-5">
                  {metadata.comments.map((comment, idx) => (
                    <div key={idx} className="space-y-1.5">
                      <div className="flex items-center gap-1.5">
                        <h4 className="text-sm font-semibold text-foreground">{comment.topic}</h4>
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <Info className="h-3.5 w-3.5 text-muted-foreground cursor-help" />
                          </TooltipTrigger>
                          <TooltipContent className="max-w-xs">
                            <p>{getCommentTooltip(comment.topic)}</p>
                          </TooltipContent>
                        </Tooltip>
                      </div>
                      <p className="text-sm text-muted-foreground leading-relaxed pl-0">
                        {comment.text}
                      </p>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Protein Features */}
            {metadata.features && metadata.features.length > 0 && (
              <div className="mt-8">
                <SectionHeader
                  label="Protein Features"
                  tooltip="Sequence annotations describing regions, sites, and domains in the protein. Includes binding sites, active sites, domains, motifs, and post-translational modification sites."
                />
                <div className="space-y-4 mt-4">
                  {metadata.features.slice(0, 20).map((feature, idx) => {
                    const cleanedDesc = cleanFeatureDescription(feature.description);
                    return (
                      <div key={idx} className="space-y-1">
                        <div className="flex items-center gap-2">
                          <Badge variant="secondary" className="shrink-0 text-xs">
                            {feature.feature_type}
                          </Badge>
                          {feature.start_pos && feature.end_pos && (
                            <span className="text-xs text-muted-foreground">
                              Position {feature.start_pos}–{feature.end_pos}
                            </span>
                          )}
                        </div>
                        {cleanedDesc && (
                          <p className="text-sm text-muted-foreground leading-relaxed pl-0">
                            {cleanedDesc}
                          </p>
                        )}
                      </div>
                    );
                  })}
                </div>
                {metadata.features.length > 20 && (
                  <p className="text-xs text-muted-foreground mt-2">
                    + {metadata.features.length - 20} more features
                  </p>
                )}
              </div>
            )}

            {/* Cross References */}
            {metadata.cross_references && metadata.cross_references.length > 0 && (
              <div className="mt-8">
                <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-3">
                  Database Cross-References
                </h3>
                <div className="grid grid-cols-2 md:grid-cols-3 gap-2">
                  {metadata.cross_references.slice(0, 30).map((xref, idx) => (
                    <div key={idx} className="flex items-center gap-2 text-sm">
                      <Badge variant="outline" className="font-mono text-xs">{xref.database}</Badge>
                      <span className="text-muted-foreground truncate">{xref.database_id}</span>
                    </div>
                  ))}
                </div>
                {metadata.cross_references.length > 30 && (
                  <p className="text-xs text-muted-foreground mt-2">
                    + {metadata.cross_references.length - 30} more references
                  </p>
                )}
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}

function InfoField({
  label,
  value,
  tooltip,
}: {
  label: string;
  value: React.ReactNode;
  tooltip?: string;
}) {
  return (
    <div className="flex items-start justify-between gap-4">
      <div className="flex items-center gap-1.5 text-sm text-muted-foreground min-w-[120px]">
        <span>{label}</span>
        {tooltip && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Info className="h-3.5 w-3.5 cursor-help" />
            </TooltipTrigger>
            <TooltipContent className="max-w-xs">
              <p>{tooltip}</p>
            </TooltipContent>
          </Tooltip>
        )}
      </div>
      <span className="font-medium text-sm text-right">{value}</span>
    </div>
  );
}

function SectionHeader({
  label,
  tooltip,
}: {
  label: string;
  tooltip?: string;
}) {
  return (
    <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-2 flex items-center gap-1.5">
      <span>{label}</span>
      {tooltip && (
        <Tooltip>
          <TooltipTrigger asChild>
            <Info className="h-3.5 w-3.5 cursor-help" />
          </TooltipTrigger>
          <TooltipContent className="max-w-xs">
            <p>{tooltip}</p>
          </TooltipContent>
        </Tooltip>
      )}
    </h3>
  );
}
