'use client';

import * as React from 'react';
import { Link } from '@/i18n/navigation';
import { useTranslations } from 'next-intl';
import { Menu, X, Github } from 'lucide-react';
import { ThemeToggle } from '@/components/shared/theme-toggle';
import { LocaleSwitcher } from '@/components/shared/locale-switcher';
import { Logo } from '@/components/shared/logo';
import { DocsSearch } from '@/components/shared/docs-search';
import { siteConfig } from '@/lib/site-config';

export function Header() {
  const [mobileMenuOpen, setMobileMenuOpen] = React.useState(false);
  const t = useTranslations('nav');

  return (
    <header className="sticky top-0 z-50 w-full border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="container flex h-16 items-center justify-between">
        {/* Logo and Navigation */}
        <div className="flex items-center gap-6">
          <Logo size="md" />
          <nav className="hidden md:flex items-center gap-6">
            <Link
              href="/docs"
              className="text-sm font-medium text-foreground/80 transition-colors hover:text-foreground"
            >
              {t('docs')}
            </Link>
          </nav>
        </div>

        {/* Desktop Right Side */}
        <div className="hidden items-center gap-2 md:flex">
          <DocsSearch />
          <LocaleSwitcher />
          <ThemeToggle />
          <Link
            href={siteConfig.github.url}
            target="_blank"
            rel="noopener noreferrer"
            className="text-muted-foreground transition-colors hover:text-foreground"
          >
            <Github className="h-5 w-5" />
            <span className="sr-only">GitHub</span>
          </Link>
        </div>

        {/* Mobile Menu Button */}
        <button
          type="button"
          className="md:hidden"
          onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
        >
          {mobileMenuOpen ? (
            <X className="h-6 w-6" />
          ) : (
            <Menu className="h-6 w-6" />
          )}
          <span className="sr-only">Toggle menu</span>
        </button>
      </div>

      {/* Mobile Navigation */}
      {mobileMenuOpen && (
        <div className="border-t md:hidden">
          <nav className="container flex flex-col gap-4 py-4">
            <DocsSearch />
            <Link
              href="/docs"
              className="text-sm font-medium text-foreground/80 transition-colors hover:text-foreground"
              onClick={() => setMobileMenuOpen(false)}
            >
              {t('docs')}
            </Link>
            <div className="flex items-center gap-2">
              <LocaleSwitcher />
              <ThemeToggle />
              <Link
                href={siteConfig.github.url}
                target="_blank"
                rel="noopener noreferrer"
                className="text-muted-foreground transition-colors hover:text-foreground"
                onClick={() => setMobileMenuOpen(false)}
              >
                <Github className="h-5 w-5" />
                <span className="sr-only">GitHub</span>
              </Link>
            </div>
          </nav>
        </div>
      )}
    </header>
  );
}
