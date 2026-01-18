import { redirect, notFound } from 'next/navigation';
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
  const { locale, org, name } = await params;

  try {
    // Fetch data source to get latest version
    const dataSource = await getDataSource(org, name);

    // Redirect to latest version
    if (dataSource.latest_version) {
      redirect(`/${locale}/sources/${org}/${name}/${dataSource.latest_version}`);
    }

    // Fallback if no latest version (shouldn't happen)
    if (dataSource.versions && dataSource.versions.length > 0) {
      redirect(`/${locale}/sources/${org}/${name}/${dataSource.versions[0].version}`);
    }

    // If no versions at all, show 404
    notFound();
  } catch (error) {
    console.error('Error fetching data source:', error);
    // Show 404 page
    notFound();
  }
}
