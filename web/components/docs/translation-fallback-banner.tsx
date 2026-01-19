'use client';

import { AlertTriangle } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { siteConfig } from '@/lib/site-config';

interface TranslationFallbackBannerProps {
  requestedLocale: string;
  fallbackLocale: string;
}

export function TranslationFallbackBanner({
  requestedLocale,
  fallbackLocale,
}: TranslationFallbackBannerProps) {
  const localeNames: Record<string, string> = {
    en: 'English',
    de: 'German',
    es: 'Spanish',
    fr: 'French',
    ja: 'Japanese',
    zh: 'Chinese',
  };

  const requestedLanguage = localeNames[requestedLocale] || requestedLocale;
  const fallbackLanguage = localeNames[fallbackLocale] || fallbackLocale;

  return (
    <Alert variant="warning" className="mb-6">
      <AlertTriangle className="h-4 w-4" />
      <AlertTitle>Translation Not Available</AlertTitle>
      <AlertDescription>
        <p className="mb-2">
          This page is not yet available in {requestedLanguage}. Showing {fallbackLanguage} content instead.
        </p>
        <p>
          Help us translate this page!{' '}
          <a
            href={`${siteConfig.github.url}/tree/main/web/app/[locale]/docs/content`}
            target="_blank"
            rel="noopener noreferrer"
            className="font-medium underline underline-offset-4 hover:text-yellow-700 dark:hover:text-yellow-300"
          >
            Contribute on GitHub
          </a>
        </p>
      </AlertDescription>
    </Alert>
  );
}
