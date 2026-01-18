import { Suspense } from 'react';
import { SourcesList } from './sources-list';

interface PageProps {
  searchParams: Promise<{
    page?: string;
    org?: string;
    type?: string;
    sort?: string;
  }>;
}

export default async function SourcesPage({ searchParams }: PageProps) {
  const params = await searchParams;

  return (
    <div className="container py-8">
      <Suspense fallback={<SourcesListSkeleton />}>
        <SourcesList searchParams={params} />
      </Suspense>
    </div>
  );
}

function SourcesListSkeleton() {
  return (
    <div className="space-y-6">
      <div className="h-12 w-64 animate-pulse rounded-lg bg-muted" />
      <div className="flex gap-4">
        <div className="h-10 w-48 animate-pulse rounded-lg bg-muted" />
        <div className="h-10 w-48 animate-pulse rounded-lg bg-muted" />
      </div>
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {[...Array(9)].map((_, i) => (
          <div key={i} className="h-48 animate-pulse rounded-lg bg-muted" />
        ))}
      </div>
    </div>
  );
}
