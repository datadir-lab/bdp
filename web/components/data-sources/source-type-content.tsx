'use client';

import * as React from 'react';
import type { DataSource, DataSourceVersion } from '@/lib/types/data-source';
import { ProteinMetadataContent } from './protein-metadata-content';
import { OrganismMetadataContent } from './organism-metadata-content';
import { GenericMetadataContent } from './generic-metadata-content';

interface SourceTypeContentProps {
  dataSource: DataSource;
  currentVersion: DataSourceVersion & { organization: string; name: string };
}

/**
 * Dynamic component that renders different content based on source_type
 */
export function SourceTypeContent({ dataSource, currentVersion }: SourceTypeContentProps) {
  const sourceType = dataSource.source_type;

  // Render different components based on source type
  switch (sourceType) {
    case 'protein':
      return (
        <ProteinMetadataContent
          dataSource={dataSource}
          currentVersion={currentVersion}
        />
      );

    case 'organism':
      return (
        <OrganismMetadataContent
          dataSource={dataSource}
          currentVersion={currentVersion}
        />
      );

    default:
      return (
        <GenericMetadataContent
          dataSource={dataSource}
          currentVersion={currentVersion}
        />
      );
  }
}
