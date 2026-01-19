import DocsInstallationEn from '../content/en/installation.mdx';
import DocsInstallationDe from '../content/de/installation.mdx';
import { locales } from '@/i18n/config';
import { loadLocalizedContent, createContentMap } from '@/lib/docs-loader';
import { TranslationFallbackBanner } from '@/components/docs/translation-fallback-banner';

export const dynamic = 'force-static';

export function generateStaticParams() {
  return locales.map((locale) => ({ locale }));
}

const contentMap = createContentMap({
  en: DocsInstallationEn,
  de: DocsInstallationDe,
});

export default async function DocsInstallation({
  params,
}: {
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;

  const { Content, loadedLocale, isFallback } = loadLocalizedContent(
    locale,
    contentMap
  );

  return (
    <>
      {isFallback && (
        <TranslationFallbackBanner
          requestedLocale={locale}
          fallbackLocale={loadedLocale}
        />
      )}
      <Content />
    </>
  );
}
