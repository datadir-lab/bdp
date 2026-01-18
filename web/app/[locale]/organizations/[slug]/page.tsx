import { Suspense } from 'react';
import { notFound } from 'next/navigation';
import { getOrganization } from '@/lib/api/organizations';
import { listDataSources } from '@/lib/api/data-sources';
import { OrganizationDetail } from './organization-detail';

interface PageProps {
  params: Promise<{
    locale: string;
    slug: string;
  }>;
  searchParams: Promise<{
    page?: string;
  }>;
}

// Generate static params for static export
// For API-driven pages, we return empty array - these will be rendered on-demand
export function generateStaticParams() {
  return [] as Array<{ slug: string }>;
}

export default async function OrganizationPage({
  params,
  searchParams,
}: PageProps) {
  const { locale, slug } = await params;
  const { page } = await searchParams;
  const currentPage = page ? parseInt(page) : 1;

  try {
    // Fetch organization details and its data sources
    const [organization, dataSourcesResult] = await Promise.all([
      getOrganization(slug),
      listDataSources({ org: slug, page: currentPage, limit: 20 }),
    ]);

    return (
      <div className="container py-8">
        <Suspense fallback={<OrganizationDetailSkeleton />}>
          <OrganizationDetail
            organization={organization}
            dataSources={dataSourcesResult.data}
            pagination={{
              currentPage,
              totalPages: dataSourcesResult.pages,
              total: dataSourcesResult.total,
            }}
            locale={locale}
          />
        </Suspense>
      </div>
    );
  } catch (error) {
    console.error('Error fetching organization:', error);
    notFound();
  }
}

function OrganizationDetailSkeleton() {
  return (
    <div className="space-y-6">
      <div className="h-32 w-full animate-pulse rounded-lg bg-muted" />
      <div className="grid gap-6 md:grid-cols-4">
        {[...Array(4)].map((_, i) => (
          <div key={i} className="h-24 animate-pulse rounded-lg bg-muted" />
        ))}
      </div>
      <div className="space-y-4">
        {[...Array(5)].map((_, i) => (
          <div key={i} className="h-32 animate-pulse rounded-lg bg-muted" />
        ))}
      </div>
    </div>
  );
}
