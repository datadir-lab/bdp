'use client';

import type { ReactNode } from 'react';
import { Link } from '@/i18n/navigation';
import { siteConfig } from '@/lib/site-config';
import { DocsSidebar } from './components/DocsSidebar';

export const dynamic = 'force-static';

// Docs layout - just provides sidebar/content structure
// Header and Footer come from the main [locale] layout
export default function DocsLayout({ children }: { children: ReactNode }) {
  return (
    <div className="container flex-1">
      <div className="flex gap-8 py-8">
        {/* Left Sidebar - Navigation */}
        <aside className="hidden md:block w-64 shrink-0">
          <div className="sticky top-24">
            <DocsSidebar />
          </div>
        </aside>

        {/* Main Content */}
        <main className="flex-1 min-w-0">
          <article
            className="max-w-none"
            data-pagefind-body
          >
            {children}
          </article>
        </main>

        {/* Right Sidebar - Actions */}
        <aside className="hidden xl:block w-64 shrink-0">
          <div className="sticky top-24 space-y-4">
            <div className="rounded-lg border p-4">
              <h3 className="font-semibold text-sm mb-3">Help us improve</h3>
              <a
                href={`${siteConfig.github.url}/edit/main/docs`}
                target="_blank"
                rel="noopener noreferrer"
                className="text-sm text-muted-foreground hover:text-foreground transition-colors"
              >
                Edit this page on GitHub
              </a>
            </div>

            <div className="rounded-lg border p-4">
              <h3 className="font-semibold text-sm mb-3">Join community</h3>
              <p className="text-sm text-muted-foreground">
                Connect with other developers and get help
              </p>
            </div>

            <div className="rounded-lg border p-4">
              <h3 className="font-semibold text-sm mb-3">Support project</h3>
              <a
                href={siteConfig.github.url}
                target="_blank"
                rel="noopener noreferrer"
                className="text-sm text-muted-foreground hover:text-foreground transition-colors"
              >
                Star on GitHub
              </a>
            </div>
          </div>
        </aside>
      </div>
    </div>
  );
}
