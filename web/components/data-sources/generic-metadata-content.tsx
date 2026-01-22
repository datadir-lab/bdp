'use client';

import * as React from 'react';
import { FileText } from 'lucide-react';
import type { DataSource, DataSourceVersion } from '@/lib/types/data-source';

interface GenericMetadataContentProps {
  dataSource: DataSource;
  currentVersion: DataSourceVersion & { organization: string; name: string };
}

export function GenericMetadataContent({
  dataSource,
}: GenericMetadataContentProps) {
  return (
    <div className="space-y-12">
      {/* Generic metadata for other types */}
      {dataSource.external_id && (
        <>
          <div>
            <div className="flex items-center gap-2 mb-6">
              <FileText className="h-5 w-5 text-muted-foreground" />
              <h2 className="text-xl font-semibold">Additional Information</h2>
            </div>

            <div className="p-5 rounded-lg border bg-card">
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
