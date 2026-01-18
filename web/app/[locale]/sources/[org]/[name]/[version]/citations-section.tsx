'use client';

import * as React from 'react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Check, Copy, ExternalLink } from 'lucide-react';
import type { Citation } from '@/lib/types/data-source';

interface CitationsSectionProps {
  citations: Citation[];
}

export function CitationsSection({ citations }: CitationsSectionProps) {
  const [copiedId, setCopiedId] = React.useState<string | null>(null);

  const handleCopy = async (text: string, id: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopiedId(id);
      setTimeout(() => setCopiedId(null), 2000);
    } catch (error) {
      console.error('Failed to copy:', error);
    }
  };

  const formatBibTeX = (citation: Citation): string => {
    const id = citation.doi?.replace(/[^a-zA-Z0-9]/g, '') || 'citation';
    const year = citation.publication_date
      ? new Date(citation.publication_date).getFullYear()
      : '';

    return `@article{${id},
  title = {${citation.title}},
  author = {${citation.authors || 'Unknown'}},
  journal = {${citation.journal || 'Unknown'}},
  year = {${year}},
  doi = {${citation.doi || ''}},
  pmid = {${citation.pubmed_id || ''}},
  url = {${citation.url || ''}}
}`;
  };

  return (
    <div>
      <h2 className="mb-4 text-xl font-semibold">Citations</h2>
      <div className="space-y-4">
        {citations.map((citation) => (
          <div key={citation.id} className="rounded-lg border bg-card p-6 space-y-4">
            <div className="space-y-2">
              <div className="flex items-start justify-between gap-4">
                <div className="flex-1">
                  <h3 className="font-semibold">{citation.title}</h3>
                  {citation.authors && (
                    <p className="text-sm text-muted-foreground mt-1">
                      {citation.authors}
                    </p>
                  )}
                </div>
                <Badge variant="outline" className="capitalize shrink-0">
                  {citation.citation_type}
                </Badge>
              </div>

              <div className="flex flex-wrap gap-2 text-sm text-muted-foreground">
                {citation.journal && <span>{citation.journal}</span>}
                {citation.publication_date && (
                  <span>
                    • {new Date(citation.publication_date).getFullYear()}
                  </span>
                )}
                {citation.doi && (
                  <a
                    href={`https://doi.org/${citation.doi}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="inline-flex items-center gap-1 text-primary hover:underline"
                  >
                    DOI: {citation.doi}
                    <ExternalLink className="h-3 w-3" />
                  </a>
                )}
                {citation.pubmed_id && (
                  <a
                    href={`https://pubmed.ncbi.nlm.nih.gov/${citation.pubmed_id}/`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="inline-flex items-center gap-1 text-primary hover:underline"
                  >
                    PMID: {citation.pubmed_id}
                    <ExternalLink className="h-3 w-3" />
                  </a>
                )}
              </div>
            </div>

            <Tabs defaultValue="formatted" className="w-full">
              <TabsList className="grid w-full max-w-md grid-cols-2">
                <TabsTrigger value="formatted">Formatted</TabsTrigger>
                <TabsTrigger value="bibtex">BibTeX</TabsTrigger>
              </TabsList>

              <TabsContent value="formatted" className="mt-4">
                <div className="rounded-lg border bg-muted/50 p-4">
                  <p className="text-sm">
                    {citation.authors && <span>{citation.authors}. </span>}
                    <span className="font-medium">{citation.title}.</span>
                    {citation.journal && <span> {citation.journal}.</span>}
                    {citation.publication_date && (
                      <span>
                        {' '}
                        {new Date(citation.publication_date).getFullYear()}.
                      </span>
                    )}
                    {citation.doi && <span> DOI: {citation.doi}.</span>}
                  </p>
                </div>
              </TabsContent>

              <TabsContent value="bibtex" className="mt-4">
                <div className="relative rounded-lg border bg-muted/50 p-4">
                  <pre className="text-sm font-mono overflow-x-auto">
                    {formatBibTeX(citation)}
                  </pre>
                  <Button
                    size="sm"
                    variant="ghost"
                    className="absolute top-2 right-2"
                    onClick={() =>
                      handleCopy(formatBibTeX(citation), `bibtex-${citation.id}`)
                    }
                  >
                    {copiedId === `bibtex-${citation.id}` ? (
                      <>
                        <Check className="h-4 w-4 mr-1" />
                        Copied
                      </>
                    ) : (
                      <>
                        <Copy className="h-4 w-4 mr-1" />
                        Copy
                      </>
                    )}
                  </Button>
                </div>
              </TabsContent>
            </Tabs>
          </div>
        ))}
      </div>
    </div>
  );
}
