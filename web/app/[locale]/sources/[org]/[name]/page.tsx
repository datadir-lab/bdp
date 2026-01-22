import { notFound } from 'next/navigation';
import { redirect } from '@/i18n/navigation';
import { getDataSource } from '@/lib/api/data-sources';

interface PageProps {
  params: Promise<{
    locale: string;
    org: string;
    name: string;
  }>;
}

// Generate static params for static export
// For API-driven pages, we return empty array - these will be rendered on-demand
export function generateStaticParams() {
  return [] as Array<{ org: string; name: string }>;
}

export default async function DataSourceRedirectPage({ params }: PageProps) {
  const resolvedParams = await params;
  const { locale, org, name } = resolvedParams;

  // Validate params - check for undefined, null, or string "undefined"
  if (!org || !name || org === 'undefined' || name === 'undefined' ||
      org.trim() === '' || name.trim() === '') {
    console.error('Invalid route params:', resolvedParams);
    // Redirect to sources list instead of a specific page
    redirect('/sources');
  }

  try {
    // Fetch data source to get latest version
    const dataSource = await getDataSource(org, name);

    // Ensure we have valid data source info
    if (!dataSource || !dataSource.slug) {
      console.error('Invalid data source response:', dataSource);
      notFound();
    }

    // Use organization slug from response to ensure correct casing
    const orgSlug = dataSource.organization?.slug || org;
    const dataSourceSlug = dataSource.slug || name;

    // Redirect to latest version
    if (dataSource.latest_version) {
      const redirectUrl = `/sources/${orgSlug}/${dataSourceSlug}/${dataSource.latest_version}`;
      console.log('[Redirect] Redirecting to:', redirectUrl);
      redirect(redirectUrl);
    }

    // Fallback if no latest version (shouldn't happen)
    if (dataSource.versions && dataSource.versions.length > 0) {
      const redirectUrl = `/sources/${orgSlug}/${dataSourceSlug}/${dataSource.versions[0].version}`;
      console.log('[Redirect] Redirecting to:', redirectUrl);
      redirect(redirectUrl);
    }

    // If no versions at all, redirect to overview page with 'latest' as version
    // This allows viewing metadata for data sources that haven't been versioned yet
    const redirectUrl = `/sources/${orgSlug}/${dataSourceSlug}/latest`;
    console.log('[Redirect] Redirecting to:', redirectUrl);
    redirect(redirectUrl);
  } catch (error) {
    // Re-throw redirect errors (Next.js redirects work by throwing errors)
    if (error && typeof error === 'object' && 'digest' in error &&
        typeof error.digest === 'string' && error.digest.startsWith('NEXT_REDIRECT')) {
      throw error;
    }

    console.error('Error fetching data source:', error);
    // Show 404 page
    notFound();
  }
}
