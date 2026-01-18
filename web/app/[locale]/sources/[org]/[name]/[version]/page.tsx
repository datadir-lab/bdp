import { Suspense } from 'react';
import { notFound } from 'next/navigation';
import { getDataSource, getDataSourceVersion } from '@/lib/api/data-sources';
import { DataSourceDetail } from './data-source-detail';

interface PageProps {
  params: Promise<{
    locale: string;
    org: string;
    name: string;
    version: string;
  }>;
}

// Generate static params for static export
// For API-driven pages, we return empty array - these will be rendered on-demand
export function generateStaticParams() {
  return [] as Array<{ org: string; name: string; version: string }>;
}

export default async function DataSourceVersionPage({ params }: PageProps) {
  const { locale, org, name, version } = await params;

  try {
    // Fetch both data source (for versions list) and specific version details
    const [dataSource, versionDetails] = await Promise.all([
      getDataSource(org, name),
      getDataSourceVersion(org, name, version),
    ]);

    return (
      <div className="container py-8">
        <Suspense fallback={<DataSourceDetailSkeleton />}>
          <DataSourceDetail
            dataSource={dataSource}
            currentVersion={versionDetails}
            locale={locale}
          />
        </Suspense>
      </div>
    );
  } catch (error) {
    console.error('Error fetching data source version:', error);
    notFound();
  }
}

function DataSourceDetailSkeleton() {
  return (
    <div className="space-y-6">
      <div className="h-12 w-2/3 animate-pulse rounded-lg bg-muted" />
      <div className="h-32 animate-pulse rounded-lg bg-muted" />
      <div className="grid gap-6 md:grid-cols-2">
        <div className="h-64 animate-pulse rounded-lg bg-muted" />
        <div className="h-64 animate-pulse rounded-lg bg-muted" />
      </div>
    </div>
  );
}
