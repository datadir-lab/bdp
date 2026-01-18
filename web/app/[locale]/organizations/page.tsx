import { Suspense } from 'react';
import { OrganizationsList } from './organizations-list';

interface PageProps {
  searchParams: Promise<{
    page?: string;
    sort?: string;
  }>;
}

export default async function OrganizationsPage({ searchParams }: PageProps) {
  const params = await searchParams;

  return (
    <div className="container py-8">
      <Suspense fallback={<OrganizationsListSkeleton />}>
        <OrganizationsList searchParams={params} />
      </Suspense>
    </div>
  );
}

function OrganizationsListSkeleton() {
  return (
    <div className="space-y-6">
      <div className="h-12 w-64 animate-pulse rounded-lg bg-muted" />
      <div className="h-10 w-48 animate-pulse rounded-lg bg-muted" />
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {[...Array(9)].map((_, i) => (
          <div key={i} className="h-32 animate-pulse rounded-lg bg-muted" />
        ))}
      </div>
    </div>
  );
}
