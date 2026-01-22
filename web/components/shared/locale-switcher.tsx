'use client';

import * as React from 'react';
import { useLocale } from 'next-intl';
import { usePathname, useRouter } from '@/i18n/navigation';
import { type Locale, locales } from '@/i18n/config';
import { Check, Globe } from 'lucide-react';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Button } from '@/components/ui/button';

interface LocaleConfig {
  code: Locale;
  label: string;
  flag: string;
  nativeName: string;
}

const LOCALE_CONFIG: Record<Locale, LocaleConfig> = {
  en: {
    code: 'en',
    label: 'English',
    flag: '🇺🇸',
    nativeName: 'English',
  },
  de: {
    code: 'de',
    label: 'German',
    flag: '🇩🇪',
    nativeName: 'Deutsch',
  },
} as const;

export function LocaleSwitcher() {
  const currentLocale = useLocale() as Locale;
  const router = useRouter();
  const pathname = usePathname();
  const [isPending, startTransition] = React.useTransition();

  const handleLocaleChange = (newLocale: Locale) => {
    startTransition(() => {
      router.replace(pathname, { locale: newLocale });
    });
  };

  const currentConfig = LOCALE_CONFIG[currentLocale];

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="outline"
          size="sm"
          disabled={isPending}
          aria-label="Switch language"
        >
          <Globe className="h-4 w-4" />
          <span className="hidden sm:inline-block ml-2">{currentConfig.flag}</span>
          <span className="sr-only">Current language: {currentConfig.nativeName}</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-[12rem]">
        <DropdownMenuLabel>Select Language</DropdownMenuLabel>
        <DropdownMenuSeparator />
        {locales.map((locale) => {
          const config = LOCALE_CONFIG[locale];
          const isSelected = locale === currentLocale;

          return (
            <DropdownMenuItem
              key={locale}
              disabled={isPending}
              onClick={() => handleLocaleChange(locale)}
              className={isSelected ? 'bg-accent' : ''}
            >
              <span className="mr-2 text-base">{config.flag}</span>
              <span className="flex-1">{config.nativeName}</span>
              {isSelected && <Check className="h-4 w-4 ml-2" />}
            </DropdownMenuItem>
          );
        })}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
