import DocsBestPracticesEn from '../content/en/best-practices.mdx';
import DocsBestPracticesDe from '../content/de/best-practices.mdx';
import { locales } from '@/i18n/config';
import { loadLocalizedContent, createContentMap } from '@/lib/docs-loader';
import { TranslationFallbackBanner } from '@/components/docs/translation-fallback-banner';

export const dynamic = 'force-static';

export function generateStaticParams() {
  return locales.map((locale) => ({ locale }));
}

const contentMap = createContentMap({
  en: DocsBestPracticesEn,
  de: DocsBestPracticesDe,
});

export default async function DocsBestPractices({
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
