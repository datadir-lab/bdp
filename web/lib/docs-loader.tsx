import { ComponentType } from 'react';
import { defaultLocale, type Locale } from '@/i18n/config';

/**
 * Result of loading localized documentation content.
 * Indicates whether fallback was used and which locale's content was loaded.
 */
export interface LocalizedContentResult {
  /** The MDX component to render */
  Content: ComponentType;
  /** The locale of the content that was loaded */
  loadedLocale: Locale;
  /** Whether the content is using fallback (not the requested locale) */
  isFallback: boolean;
}

/**
 * Loads localized MDX content with automatic fallback to English.
 *
 * Uses a switch statement for clear, maintainable locale handling.
 * Falls back to English (defaultLocale) when translation is unavailable.
 *
 * @param locale - The requested locale
 * @param contentMap - Map of locale to MDX component
 * @returns Result containing the content component and fallback info
 *
 * @example
 * ```tsx
 * import DocsIntroductionEn from './content/en/introduction.mdx';
 * import DocsIntroductionDe from './content/de/introduction.mdx';
 *
 * const result = loadLocalizedContent(locale, {
 *   en: DocsIntroductionEn,
 *   de: DocsIntroductionDe,
 * });
 * ```
 */
export function loadLocalizedContent(
  locale: string,
  contentMap: Partial<Record<Locale, ComponentType>>
): LocalizedContentResult {
  // Attempt to load content for the requested locale
  let Content: ComponentType | undefined;

  switch (locale as Locale) {
    case 'en':
      Content = contentMap.en;
      break;
    case 'de':
      Content = contentMap.de;
      break;
    // Add more locales here as they become available
    // case 'es':
    //   Content = contentMap.es;
    //   break;
    // case 'fr':
    //   Content = contentMap.fr;
    //   break;
    default:
      Content = undefined;
  }

  // If content exists for the requested locale, return it
  if (Content) {
    return {
      Content,
      loadedLocale: locale as Locale,
      isFallback: false,
    };
  }

  // Fallback to default locale (English)
  const fallbackContent = contentMap[defaultLocale];

  if (!fallbackContent) {
    throw new Error(
      `No content available for locale "${locale}" and fallback locale "${defaultLocale}" is also missing. ` +
      `Please ensure content exists for at least the default locale.`
    );
  }

  return {
    Content: fallbackContent,
    loadedLocale: defaultLocale,
    isFallback: true,
  };
}

/**
 * Type-safe helper to create content maps for documentation pages.
 * Ensures that at minimum, English content is provided.
 *
 * @example
 * ```tsx
 * const contentMap = createContentMap({
 *   en: DocsIntroductionEn,
 *   de: DocsIntroductionDe,
 * });
 * ```
 */
export function createContentMap<T extends ComponentType>(
  map: { en: T } & Partial<Record<Exclude<Locale, 'en'>, T>>
): Record<Locale, T | undefined> {
  return map as Record<Locale, T | undefined>;
}
