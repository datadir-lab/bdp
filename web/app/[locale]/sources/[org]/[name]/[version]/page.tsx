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
    // Fetch data source first
    const dataSource = await getDataSource(org, name);

    // Handle 'latest' version for data sources with no versions
    if (version === 'latest' && dataSource.versions.length === 0) {
      // Return minimal view for data sources without versions
      return (
        <div className="container py-8">
          <div className="space-y-6">
            <div>
              <h1 className="text-3xl font-bold">{dataSource.name}</h1>
              <p className="text-muted-foreground mt-2">{dataSource.slug}</p>
            </div>
            {dataSource.description && (
              <p className="text-lg">{dataSource.description}</p>
            )}
            <div className="rounded-lg border bg-muted/50 p-6 text-center">
              <p className="text-muted-foreground">No versions available yet for this data source.</p>
            </div>
          </div>
        </div>
      );
    }

    // Fetch specific version details
    const versionDetails = await getDataSourceVersion(org, name, version);

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
    console.error('API URL:', process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000');
    console.error('Params:', { org, name, version });

    // Check if it's a network error vs actual 404
    const isNetworkError = error && typeof error === 'object' &&
      ('code' in error && error.code === 'NETWORK_ERROR');

    const is404 = error && typeof error === 'object' &&
      ('status' in error && error.status === 404);

    // Show network error message
    if (isNetworkError) {
      return (
        <div className="container py-8">
          <div className="space-y-6">
            <div className="rounded-lg border border-destructive bg-destructive/10 p-6">
              <h2 className="text-lg font-semibold mb-2">Cannot Connect to API</h2>
              <p className="text-sm text-muted-foreground mb-4">
                Please check that the backend server is running on {process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000'}
              </p>
              <button
                onClick={() => window.location.reload()}
                className="px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90"
              >
                Retry
              </button>
              <details className="mt-4 text-xs">
                <summary className="cursor-pointer font-medium">Technical Details</summary>
                <pre className="mt-2 p-2 bg-black/5 rounded overflow-auto">
                  {JSON.stringify({ org, name, version, error: String(error) }, null, 2)}
                </pre>
              </details>
            </div>
          </div>
        </div>
      );
    }

    // If it's a 'latest' version with no versions, show a friendly message
    if (version === 'latest' && !is404) {
      return (
        <div className="container py-8">
          <div className="space-y-6">
            <div className="rounded-lg border border-destructive bg-destructive/10 p-6">
              <h2 className="text-lg font-semibold mb-2">Unable to Load Data Source</h2>
              <p className="text-sm text-muted-foreground mb-4">
                There was an error loading this data source.
              </p>
              <details className="text-xs">
                <summary className="cursor-pointer font-medium">Technical Details</summary>
                <pre className="mt-2 p-2 bg-black/5 rounded overflow-auto">
                  {JSON.stringify({ org, name, version, error: String(error) }, null, 2)}
                </pre>
              </details>
            </div>
          </div>
        </div>
      );
    }

    // Only show 404 for actual not-found errors
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
