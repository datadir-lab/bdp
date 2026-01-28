'use client';

import { Suspense, useEffect, useState } from 'react';
import { useParams } from 'next/navigation';
import { notFound } from 'next/navigation';
import { getDataSource, getDataSourceVersion } from '@/lib/api/data-sources';
import { DataSourceDetail } from './data-source-detail';
import type { DataSource, DataSourceVersion } from '@/lib/types/data-source';
import type { ApiError } from '@/lib/types';

export default function DataSourceVersionPage() {
  const params = useParams();
  const { locale, org, name, version } = params as { locale: string; org: string; name: string; version: string };

  const [dataSource, setDataSource] = useState<DataSource | null>(null);
  const [versionDetails, setVersionDetails] = useState<(DataSourceVersion & { organization: string; name: string }) | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<ApiError | Error | null>(null);

  useEffect(() => {
    async function fetchData() {
      setIsLoading(true);
      setError(null);

      try {
        console.log('Fetching data source:', { org, name, version });

        // Fetch data source first
        const ds = await getDataSource(org, name);
        setDataSource(ds);

        // Handle 'latest' version for data sources with no versions
        if (version === 'latest' && ds.versions.length === 0) {
          setIsLoading(false);
          return;
        }

        // Fetch specific version details
        const vd = await getDataSourceVersion(org, name, version);
        setVersionDetails(vd);
        setIsLoading(false);
      } catch (err) {
        console.error('Error fetching data:', err);
        setError(err);
        setIsLoading(false);
      }
    }

    fetchData();
  }, [org, name, version]);

  if (isLoading) {
    return (
      <div className="container py-8">
        <DataSourceDetailSkeleton />
      </div>
    );
  }

  if (error) {
    const isNetworkError = error && typeof error === 'object' &&
      ('code' in error && error.code === 'NETWORK_ERROR');

    const is404 = error && typeof error === 'object' &&
      ('status' in error && error.status === 404);

    const is500 = error && typeof error === 'object' &&
      ('status' in error && error.status === 500);

    // Show 500 server error message
    if (is500) {
      const errorMessage = error.message || 'A server error occurred';
      return (
        <div className="container py-8">
          <div className="space-y-6">
            <div className="rounded-lg border border-destructive bg-destructive/10 p-6">
              <h2 className="text-lg font-semibold mb-2">Server Error</h2>
              <p className="text-sm text-muted-foreground mb-4">
                The server encountered an error while processing this data source. The data source may not exist or there may be an issue with the database.
              </p>
              <p className="text-sm font-medium mb-4">
                Error: {errorMessage}
              </p>
              <button
                onClick={() => window.location.href = `/${locale}/sources`}
                className="px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90"
              >
                Go to Data Sources
              </button>
              <details className="mt-4 text-xs">
                <summary className="cursor-pointer font-medium">Technical Details</summary>
                <pre className="mt-2 p-2 bg-black/5 rounded overflow-auto">
                  {JSON.stringify({ org, name, version, error: error }, null, 2)}
                </pre>
              </details>
            </div>
          </div>
        </div>
      );
    }

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

    // Show 404 for actual not-found errors
    if (is404) {
      return (
        <div className="container py-8">
          <div className="space-y-6">
            <div className="rounded-lg border bg-muted p-6 text-center">
              <h2 className="text-lg font-semibold mb-2">Data Source Not Found</h2>
              <p className="text-sm text-muted-foreground">
                The data source version you're looking for doesn't exist.
              </p>
            </div>
          </div>
        </div>
      );
    }

    // Generic error
    return (
      <div className="container py-8">
        <div className="rounded-lg border border-destructive bg-destructive/10 p-6">
          <h2 className="text-lg font-semibold mb-2">Error</h2>
          <p className="text-sm text-muted-foreground mb-4">
            An unexpected error occurred.
          </p>
          <details className="text-xs">
            <summary className="cursor-pointer font-medium">Technical Details</summary>
            <pre className="mt-2 p-2 bg-black/5 rounded overflow-auto">
              {JSON.stringify({ org, name, version, error: String(error) }, null, 2)}
            </pre>
          </details>
        </div>
      </div>
    );
  }

  // Helper to extract clean name without organism suffix
  const getCleanName = (name: string) => {
    return name.replace(/\s*\[.*?\]\s*$/g, '').trim();
  };

  // Handle no versions
  if (dataSource && version === 'latest' && dataSource.versions.length === 0) {
    return (
      <div className="container py-8">
        <div className="space-y-6">
          <div>
            <h1 className="text-3xl font-bold">{getCleanName(dataSource.name)}</h1>
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

  // Render the detail page
  if (dataSource && versionDetails) {
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
  }

  return null;
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
