# Next.js Frontend Development

Guide for developing the BDP web interface using **Next.js 16** and Nextra.

## Project Structure

```
web/
├── package.json
├── next.config.js
├── tsconfig.json
├── tailwind.config.ts
├── theme.config.tsx      # Nextra theme config
├── app/                  # App Router (Next.js 16)
│   ├── layout.tsx
│   ├── page.tsx          # Homepage
│   ├── packages/
│   │   ├── page.tsx      # Package list
│   │   └── [name]/
│   │       ├── page.tsx  # Package detail
│   │       └── [version]/
│   │           └── page.tsx
│   ├── search/
│   │   └── page.tsx
│   └── api/              # API routes (optional)
│       └── auth/
├── components/
│   ├── PackageCard.tsx
│   ├── VersionSelector.tsx
│   ├── SearchBar.tsx
│   └── DependencyTree.tsx
├── lib/
│   ├── api.ts            # API client
│   ├── types.ts
│   └── utils.ts
├── pages/                # Nextra docs
│   ├── _meta.json
│   ├── index.mdx
│   ├── docs/
│   │   ├── _meta.json
│   │   ├── getting-started.mdx
│   │   └── cli-reference.mdx
│   └── _app.tsx
└── public/
    └── logo.svg
```

## Next.js 16 Configuration

### next.config.js

```javascript
const withNextra = require('nextra')({
  theme: 'nextra-theme-docs',
  themeConfig: './theme.config.tsx',
});

/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  env: {
    NEXT_PUBLIC_API_URL: process.env.NEXT_PUBLIC_API_URL || 'https://api.bdp.dev/v1',
  },
  async rewrites() {
    return [
      {
        source: '/api/:path*',
        destination: `${process.env.API_URL || 'http://localhost:8000'}/api/v1/:path*`,
      },
    ];
  },
  // Next.js 16 optimizations
  experimental: {
    optimizePackageImports: ['@radix-ui/react-icons'],
  },
};

module.exports = withNextra(nextConfig);
```

### package.json

```json
{
  "name": "bdp-web",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "dev": "next dev --turbopack",
    "build": "next build",
    "start": "next start",
    "lint": "next lint",
    "type-check": "tsc --noEmit"
  },
  "dependencies": {
    "next": "^16.0.0",
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "nextra": "^3.0.0",
    "nextra-theme-docs": "^3.0.0",
    "swr": "^2.2.5",
    "tailwindcss": "^3.4.1",
    "@radix-ui/react-dialog": "^1.0.5",
    "@radix-ui/react-dropdown-menu": "^2.0.6",
    "@radix-ui/react-select": "^2.0.0",
    "clsx": "^2.1.0",
    "date-fns": "^3.0.0"
  },
  "devDependencies": {
    "@types/node": "^22",
    "@types/react": "^19",
    "@types/react-dom": "^19",
    "typescript": "^5.6",
    "autoprefixer": "^10.4.17",
    "postcss": "^8.4.33"
  }
}
```

## App Router Structure (Next.js 16)

### app/layout.tsx (Root Layout)

```typescript
import type { Metadata } from 'next';
import { Inter } from 'next/font/google';
import './globals.css';
import { Header } from '@/components/Header';
import { Footer } from '@/components/Footer';

const inter = Inter({ subsets: ['latin'] });

export const metadata: Metadata = {
  title: 'BDP - Bioinformatics Dependencies Platform',
  description: 'Package registry and environment manager for bioinformatics',
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className={inter.className}>
        <div className="flex flex-col min-h-screen">
          <Header />
          <main className="flex-1">{children}</main>
          <Footer />
        </div>
      </body>
    </html>
  );
}
```

### app/page.tsx (Homepage)

```typescript
import Link from 'next/link';
import { SearchBar } from '@/components/SearchBar';
import { RecentPackages } from '@/components/RecentPackages';

export default async function HomePage() {
  return (
    <div className="container mx-auto px-4 py-16">
      <div className="text-center mb-12">
        <h1 className="text-5xl font-bold mb-4">
          Bioinformatics Dependencies Platform
        </h1>
        <p className="text-xl text-gray-600 mb-8">
          Reproducible environments for bioinformatics research
        </p>

        <SearchBar />

        <div className="mt-8 flex gap-4 justify-center">
          <Link
            href="/docs/getting-started"
            className="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
          >
            Get Started
          </Link>
          <Link
            href="/packages"
            className="px-6 py-3 border border-gray-300 rounded-lg hover:bg-gray-50"
          >
            Browse Packages
          </Link>
        </div>
      </div>

      <RecentPackages />
    </div>
  );
}
```

### app/packages/page.tsx (Server Component)

```typescript
import { Suspense } from 'react';
import { PackageCard } from '@/components/PackageCard';
import { PackageListSkeleton } from '@/components/Skeletons';
import { getPackages } from '@/lib/api';

export default async function PackagesPage({
  searchParams,
}: {
  searchParams: { page?: string };
}) {
  const page = Number(searchParams.page) || 1;

  return (
    <div className="container mx-auto px-4 py-8">
      <h1 className="text-3xl font-bold mb-8">All Packages</h1>

      <Suspense fallback={<PackageListSkeleton />}>
        <PackageList page={page} />
      </Suspense>
    </div>
  );
}

async function PackageList({ page }: { page: number }) {
  const { data: packages, meta } = await getPackages({ page, per_page: 20 });

  return (
    <div>
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {packages.map((pkg) => (
          <PackageCard key={pkg.id} package={pkg} />
        ))}
      </div>

      {/* Pagination */}
      <div className="mt-8 flex justify-center gap-2">
        {page > 1 && (
          <a href={`/packages?page=${page - 1}`} className="px-4 py-2 border rounded">
            Previous
          </a>
        )}
        <span className="px-4 py-2">Page {page}</span>
        <a href={`/packages?page=${page + 1}`} className="px-4 py-2 border rounded">
          Next
        </a>
      </div>
    </div>
  );
}
```

### app/packages/[name]/page.tsx (Dynamic Route)

```typescript
import { notFound } from 'next/navigation';
import { getPackage, getVersions } from '@/lib/api';
import { VersionSelector } from '@/components/VersionSelector';
import { DependencyTree } from '@/components/DependencyTree';
import { InstallCommand } from '@/components/InstallCommand';

export async function generateMetadata({ params }: { params: { name: string } }) {
  const pkg = await getPackage(params.name);

  return {
    title: `${pkg.name} - BDP`,
    description: pkg.description,
  };
}

export default async function PackagePage({ params }: { params: { name: string } }) {
  const [pkg, versions] = await Promise.all([
    getPackage(params.name),
    getVersions(params.name),
  ]);

  if (!pkg) {
    notFound();
  }

  const latestVersion = versions[0];

  return (
    <div className="container mx-auto px-4 py-8">
      <div className="mb-8">
        <h1 className="text-4xl font-bold mb-2">{pkg.name}</h1>
        <p className="text-gray-600 text-lg">{pkg.description}</p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
        <div className="lg:col-span-2">
          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">Installation</h2>
            <InstallCommand packageName={pkg.name} version={latestVersion.version} />
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">Dependencies</h2>
            <DependencyTree packageName={pkg.name} version={latestVersion.version} />
          </section>
        </div>

        <aside>
          <div className="sticky top-4">
            <div className="border rounded-lg p-6 mb-4">
              <h3 className="font-semibold mb-4">Versions</h3>
              <VersionSelector versions={versions} current={latestVersion.version} />
            </div>

            <div className="border rounded-lg p-6">
              <h3 className="font-semibold mb-4">Details</h3>
              <dl className="space-y-2 text-sm">
                <div>
                  <dt className="text-gray-500">License</dt>
                  <dd>{pkg.license || 'Not specified'}</dd>
                </div>
                <div>
                  <dt className="text-gray-500">Repository</dt>
                  <dd>
                    {pkg.repository_url && (
                      <a href={pkg.repository_url} className="text-blue-600 hover:underline">
                        GitHub
                      </a>
                    )}
                  </dd>
                </div>
                <div>
                  <dt className="text-gray-500">Downloads</dt>
                  <dd>{pkg.downloads_total.toLocaleString()}</dd>
                </div>
              </dl>
            </div>
          </div>
        </aside>
      </div>
    </div>
  );
}
```

## API Client

### lib/api.ts

```typescript
const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000/api/v1';

export interface Package {
  id: string;
  name: string;
  description: string | null;
  repository_url: string | null;
  license: string | null;
  downloads_total: number;
  created_at: string;
  updated_at: string;
}

export interface Version {
  id: string;
  version: string;
  checksum: string;
  download_url: string;
  published_at: string;
  yanked: boolean;
}

interface ApiResponse<T> {
  success: boolean;
  data: T;
  meta?: {
    pagination?: {
      page: number;
      per_page: number;
      total: number;
    };
  };
}

async function fetchApi<T>(endpoint: string): Promise<ApiResponse<T>> {
  const response = await fetch(`${API_URL}${endpoint}`, {
    next: { revalidate: 60 }, // Revalidate every 60 seconds
  });

  if (!response.ok) {
    throw new Error(`API error: ${response.statusText}`);
  }

  return response.json();
}

export async function getPackages(params: {
  page?: number;
  per_page?: number;
}): Promise<ApiResponse<Package[]>> {
  const query = new URLSearchParams({
    page: String(params.page || 1),
    per_page: String(params.per_page || 20),
  });

  return fetchApi(`/packages?${query}`);
}

export async function getPackage(name: string): Promise<Package> {
  const response = await fetchApi<Package>(`/packages/${name}`);
  return response.data;
}

export async function getVersions(name: string): Promise<Version[]> {
  const response = await fetchApi<Version[]>(`/packages/${name}/versions`);
  return response.data;
}

export async function searchPackages(query: string): Promise<Package[]> {
  const response = await fetchApi<Package[]>(`/search?q=${encodeURIComponent(query)}`);
  return response.data;
}
```

## Client Components with SWR

### components/SearchBar.tsx

```typescript
'use client';

import { useState } from 'react';
import { useRouter } from 'next/navigation';
import { MagnifyingGlassIcon } from '@radix-ui/react-icons';

export function SearchBar() {
  const [query, setQuery] = useState('');
  const router = useRouter();

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (query.trim()) {
      router.push(`/search?q=${encodeURIComponent(query)}`);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="max-w-2xl mx-auto">
      <div className="relative">
        <MagnifyingGlassIcon className="absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-400" />
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search packages..."
          className="w-full pl-12 pr-4 py-4 text-lg border-2 border-gray-200 rounded-lg focus:border-blue-500 focus:outline-none"
        />
      </div>
    </form>
  );
}
```

### components/RecentPackages.tsx

```typescript
'use client';

import useSWR from 'swr';
import { PackageCard } from './PackageCard';
import { Package } from '@/lib/api';

const fetcher = (url: string) => fetch(url).then((r) => r.json()).then((d) => d.data);

export function RecentPackages() {
  const { data: packages, error, isLoading } = useSWR<Package[]>(
    '/api/packages?per_page=6',
    fetcher
  );

  if (isLoading) return <div>Loading...</div>;
  if (error) return <div>Failed to load packages</div>;
  if (!packages) return null;

  return (
    <section className="mt-16">
      <h2 className="text-3xl font-bold mb-8">Recent Packages</h2>
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {packages.map((pkg) => (
          <PackageCard key={pkg.id} package={pkg} />
        ))}
      </div>
    </section>
  );
}
```

## Nextra Documentation

### theme.config.tsx

```typescript
import { DocsThemeConfig } from 'nextra-theme-docs';

const config: DocsThemeConfig = {
  logo: <span className="font-bold">BDP Documentation</span>,
  project: {
    link: 'https://github.com/biodir/bdp',
  },
  docsRepositoryBase: 'https://github.com/biodir/bdp/tree/main/web/pages',
  footer: {
    text: 'BDP - Bioinformatics Dependencies Platform',
  },
  search: {
    placeholder: 'Search documentation...',
  },
  toc: {
    title: 'On This Page',
  },
};

export default config;
```

### pages/docs/getting-started.mdx

```mdx
# Getting Started

Welcome to BDP! This guide will help you get started with using BDP for your bioinformatics projects.

## Installation

Install the BDP CLI:

```bash
curl -sSL https://get.bdp.dev | sh
```

Or download from [releases](https://github.com/biodir/bdp/releases).

## Initialize a Project

```bash
bdp init
```

This creates a `bdp.toml` file:

```toml
[package]
name = "my-project"
version = "0.1.0"

[dependencies]
samtools = "^1.18"
```

## Install Dependencies

```bash
bdp install
```

This generates a `bdp.lock` file with exact versions.
```

## Best Practices

### 1. Use Server Components by Default

```typescript
// ✅ Server Component (default in App Router)
export default async function PackagesPage() {
  const packages = await getPackages();
  return <PackageList packages={packages} />;
}

// Only use 'use client' when needed
'use client';
export function InteractiveSearch() {
  const [query, setQuery] = useState('');
  // ...
}
```

### 2. Streaming and Suspense

```typescript
import { Suspense } from 'react';

export default function Page() {
  return (
    <Suspense fallback={<Skeleton />}>
      <SlowComponent />
    </Suspense>
  );
}
```

### 3. Type Safety

```typescript
// Define types in lib/types.ts
export interface Package {
  id: string;
  name: string;
  // ...
}

// Use throughout the app
import { Package } from '@/lib/types';
```

### 4. Error Handling

```typescript
// app/error.tsx
'use client';

export default function Error({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  return (
    <div>
      <h2>Something went wrong!</h2>
      <button onClick={() => reset()}>Try again</button>
    </div>
  );
}
```

## Resources

- [Next.js 15 Documentation](https://nextjs.org/docs)
- [React Server Components](https://nextjs.org/docs/app/building-your-application/rendering/server-components)
- [Nextra Documentation](https://nextra.site/)
- [Tailwind CSS](https://tailwindcss.com/)
- [Radix UI](https://www.radix-ui.com/)

---

**Next**: See [Testing Strategy](./testing.md) for testing approaches.
