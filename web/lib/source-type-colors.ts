export const SOURCE_TYPE_COLORS: Record<string, string> = {
  protein:             '#3b82f6',
  genome:              '#22c55e',
  annotation:          '#f97316',
  structure:           '#06b6d4',
  predicted_structure: '#0891b2',
  taxonomy:            '#a855f7',
  transcript:          '#84cc16',
  domain:              '#f59e0b',
  ontology_term:       '#8b5cf6',
  pathway:             '#10b981',
  interaction:         '#ef4444',
  variant:             '#f43f5e',
  compound:            '#d946ef',
  expression:          '#14b8a6',
  metagenome:          '#78716c',
  literature:          '#e2e8f0',
  tool:                '#64748b',
};

export const DEFAULT_POINT_COLOR = '#94a3b8';

export function getSourceTypeColor(sourceType: string | null | undefined): string {
  if (!sourceType) return DEFAULT_POINT_COLOR;
  return SOURCE_TYPE_COLORS[sourceType] ?? DEFAULT_POINT_COLOR;
}
