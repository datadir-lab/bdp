import { Suspense } from 'react';
import { SearchResults } from './search-results';

export default function SearchPage() {
  return (
    <div className="container py-8">
      <Suspense fallback={<SearchResultsSkeleton />}>
        <SearchResults />
      </Suspense>
    </div>
  );
}

function SearchResultsSkeleton() {
  return (
    <div className="space-y-6">
      <div className="h-12 w-64 animate-pulse rounded-lg bg-muted" />
      <div className="space-y-4">
        {[...Array(5)].map((_, i) => (
          <div key={i} className="h-32 animate-pulse rounded-lg bg-muted" />
        ))}
      </div>
    </div>
  );
}
