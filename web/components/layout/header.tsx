'use client';

import * as React from 'react';
import { Link } from '@/i18n/navigation';
import { useTranslations } from 'next-intl';
import { Github, BookOpen, Activity } from 'lucide-react';
import { ThemeToggle } from '@/components/shared/theme-toggle';
import { LocaleSwitcher } from '@/components/shared/locale-switcher';
import { Logo } from '@/components/shared/logo';
import { DocsSearch } from '@/components/shared/docs-search';
import { siteConfig } from '@/lib/site-config';

export function Header() {
  const t = useTranslations('nav');

  return (
    <header className="sticky top-0 z-[200] w-full border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="container flex h-16 items-center justify-between">
        {/* Left Side - Logo and Navigation */}
        <div className="flex items-center gap-4 md:gap-6">
          <Logo size="md" />

          {/* Documentation Link - Icon only on mobile, Icon + Text on desktop */}
          <Link
            href="/docs"
            className="flex items-center gap-2 text-sm font-medium text-muted-foreground transition-colors hover:text-foreground"
            title={t('docs')}
          >
            <BookOpen className="h-5 w-5 flex-shrink-0 pt-[1px]" />
            <span className="hidden md:inline leading-none">{t('docs')}</span>
          </Link>

          {/* Jobs Link - Icon only on mobile, Icon + Text on desktop */}
          <Link
            href="/jobs"
            className="flex items-center gap-2 text-sm font-medium text-muted-foreground transition-colors hover:text-foreground"
            title={t('jobs')}
          >
            <Activity className="h-5 w-5 flex-shrink-0 pt-[1px]" />
            <span className="hidden md:inline leading-none">{t('jobs')}</span>
          </Link>
        </div>

        {/* Right Side - Always Visible */}
        <div className="flex items-center gap-2">
          {/* Hide DocsSearch on mobile, show on desktop */}
          <div className="hidden md:block">
            <DocsSearch />
          </div>

          <LocaleSwitcher />
          <ThemeToggle />
          <Link
            href={siteConfig.github.url}
            target="_blank"
            rel="noopener noreferrer"
            className="pl-2 text-muted-foreground transition-colors hover:text-foreground"
            title="GitHub"
          >
            <Github className="h-5 w-5" />
            <span className="sr-only">GitHub</span>
          </Link>
        </div>
      </div>
    </header>
  );
}
