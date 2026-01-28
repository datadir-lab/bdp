import { redirect } from 'next/navigation';

interface PageProps {
  params: Promise<{
    locale: string;
    slug: string;
  }>;
}

export default async function OrgRedirectPage({ params }: PageProps) {
  const { locale, slug } = await params;

  // Redirect to the full organizations page
  redirect(`/${locale}/organizations/${slug}`);
}

export async function generateStaticParams() {
  return [];
}
