import * as React from 'react';
import { Suspense } from 'react';
import { Metadata } from 'next';
import { JobsDashboard } from './jobs-dashboard';
import { Loader2 } from 'lucide-react';

export const metadata: Metadata = {
  title: 'Ingestion Jobs',
  description: 'Monitor data ingestion jobs across all organizations',
};

function JobsPageSkeleton() {
  return (
    <div className="space-y-6">
      {/* Header Skeleton */}
      <div className="space-y-2">
        <div className="h-9 w-48 bg-muted animate-pulse rounded" />
        <div className="h-5 w-96 bg-muted animate-pulse rounded" />
      </div>

      {/* Filters Skeleton */}
      <div className="flex items-center gap-4">
        <div className="h-10 w-[150px] bg-muted animate-pulse rounded" />
        <div className="h-5 w-32 bg-muted animate-pulse rounded" />
      </div>

      {/* Tabs Skeleton */}
      <div className="h-10 w-64 bg-muted animate-pulse rounded" />

      {/* Cards Skeleton */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {[1, 2, 3].map((i) => (
          <div
            key={i}
            className="h-[400px] rounded-lg border bg-card animate-pulse"
          />
        ))}
      </div>
    </div>
  );
}

export default function JobsPage() {
  return (
    <div className="container py-8">
      <Suspense fallback={<JobsPageSkeleton />}>
        <JobsDashboard />
      </Suspense>
    </div>
  );
}
