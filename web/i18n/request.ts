import { getRequestConfig } from 'next-intl/server';
import { routing } from './routing';

export default getRequestConfig(async ({ requestLocale }) => {
  let locale = await requestLocale;

  if (!locale) {
    locale = routing.defaultLocale;
  }

  // Validate locale
  const isValidLocale = (loc: string): loc is (typeof routing.locales)[number] =>
    (routing.locales as readonly string[]).includes(loc);

  if (!isValidLocale(locale)) {
    locale = routing.defaultLocale;
  }

  return {
    locale,
    messages: (await import(`../messages/${locale}.json`)).default,
  };
});
