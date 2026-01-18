import DocsIntroductionEn from './content/en/introduction.mdx';
import DocsIntroductionDe from './content/de/introduction.mdx';
import { locales } from '@/i18n/config';

// Force static generation of docs pages
export const dynamic = 'force-static';

// Generate static params for all locales
export function generateStaticParams() {
  return locales.map((locale) => ({ locale }));
}

export default async function DocsIntroduction({
  params,
}: {
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;

  // Select the appropriate MDX content based on locale
  const Content = locale === 'de' ? DocsIntroductionDe : DocsIntroductionEn;

  return <Content />;
}
