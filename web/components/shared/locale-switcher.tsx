'use client';

import * as React from 'react';
import * as DropdownMenuPrimitive from '@radix-ui/react-dropdown-menu';
import { useLocale } from 'next-intl';
import { usePathname, useRouter } from '@/i18n/navigation';
import { type Locale, locales } from '@/i18n/config';
import { Check, Globe } from 'lucide-react';
import { cn } from '@/lib/utils';

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
  const [open, setOpen] = React.useState(false);
  const [isPending, startTransition] = React.useTransition();
  const dropdownRef = React.useRef<HTMLDivElement>(null);

  React.useEffect(() => {
    if (!open) return;

    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
    };

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setOpen(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    document.addEventListener('keydown', handleEscape);

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleEscape);
    };
  }, [open]);

  const handleLocaleChange = (newLocale: Locale) => {
    setOpen(false);
    startTransition(() => {
      router.replace(pathname, { locale: newLocale });
    });
  };

  const currentConfig = LOCALE_CONFIG[currentLocale];

  return (
    <div ref={dropdownRef} className="relative">
      <button
        type="button"
        disabled={isPending}
        onClick={() => setOpen(!open)}
        className={cn(
          'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium',
          'transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2',
          'disabled:pointer-events-none disabled:opacity-50',
          'border border-input bg-background hover:bg-accent hover:text-accent-foreground',
          'h-9 px-3'
        )}
        aria-label="Switch language"
        aria-expanded={open}
      >
        <Globe className="h-4 w-4" />
        <span className="hidden sm:inline-block">{currentConfig.flag}</span>
        <span className="sr-only">Current language: {currentConfig.nativeName}</span>
      </button>

      {open && (
        <div
          className={cn(
            'absolute right-0 top-full mt-1 z-[100] min-w-[12rem] overflow-hidden rounded-md border bg-popover p-1 text-popover-foreground shadow-md',
            'animate-in fade-in-0 zoom-in-95 slide-in-from-top-2'
          )}
        >
          <div className="px-2 py-1.5 text-sm font-semibold">
            Select Language
          </div>
          <div className="-mx-1 my-1 h-px bg-muted" />

          {locales.map((locale) => {
            const config = LOCALE_CONFIG[locale];
            const isSelected = locale === currentLocale;

            return (
              <button
                key={locale}
                type="button"
                disabled={isPending}
                onClick={() => handleLocaleChange(locale)}
                className={cn(
                  'w-full relative flex cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none',
                  'transition-colors hover:bg-accent hover:text-accent-foreground focus:bg-accent focus:text-accent-foreground',
                  'disabled:pointer-events-none disabled:opacity-50',
                  isSelected && 'bg-accent'
                )}
              >
                <span className="mr-2 text-base">{config.flag}</span>
                <span className="flex-1 text-left">{config.nativeName}</span>
                {isSelected && (
                  <Check className="h-4 w-4 ml-2" />
                )}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
