'use client';

import Link from 'next/link';
import { FileQuestion, Home, Search } from 'lucide-react';
import { ThemeProvider } from '@/components/providers';
import { HeaderStandalone } from '@/components/layout/header-standalone';
import { FooterStandalone } from '@/components/layout/footer-standalone';

export function NotFoundContent() {
  return (
    <ThemeProvider
      attribute="class"
      defaultTheme="system"
      enableSystem
      disableTransitionOnChange
    >
      <div className="relative flex min-h-screen flex-col">
        <HeaderStandalone />

        <main className="flex-1 flex items-center justify-center">
          <div className="container">
            <div className="mx-auto max-w-md text-center space-y-6 py-20">
              <div className="flex justify-center">
                <FileQuestion className="h-24 w-24 text-muted-foreground" />
              </div>

              <div className="space-y-2">
                <h1 className="text-4xl font-bold tracking-tight">404</h1>
                <h2 className="text-2xl font-semibold">Page Not Found</h2>
                <p className="text-muted-foreground">
                  The page you're looking for doesn't exist or has been moved.
                </p>
              </div>

              <div className="flex flex-col sm:flex-row gap-3 justify-center pt-4">
                <Link
                  href="/en"
                  className="inline-flex items-center justify-center rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 bg-primary text-primary-foreground hover:bg-primary/90 h-10 px-4 py-2"
                >
                  <Home className="mr-2 h-4 w-4" />
                  Go Home
                </Link>
                <Link
                  href="/en/search"
                  className="inline-flex items-center justify-center rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 border border-input bg-background hover:bg-accent hover:text-accent-foreground h-10 px-4 py-2"
                >
                  <Search className="mr-2 h-4 w-4" />
                  Search
                </Link>
              </div>
            </div>
          </div>
        </main>

        <FooterStandalone />
      </div>
    </ThemeProvider>
  );
}
