'use client';

import { Link } from '@/i18n/navigation';
import { useTranslations } from 'next-intl';
import { useState, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { SearchBar } from '@/components/search/search-bar';
import { SearchFilters } from '@/components/search/search-filters';
import { Package, Download, Lock, Handshake, HardDrive, BookText, Shield, SearchIcon, Zap, Github, Star, Building2 } from 'lucide-react';
import { GrainGradient } from '@/components/shared/grain-gradient';
import { Logo } from '@/components/shared/logo';
import { GettingStarted } from '@/components/shared/getting-started';
import { SearchFilters as SearchFiltersType } from '@/lib/types/search';
import { apiClient } from '@/lib/api-client';

interface Stats {
  datasources: number | null;
  organizations: number | null;
  downloads: number | null;
}

export default function HomePage() {
  const t = useTranslations();
  const [isFiltersOpen, setIsFiltersOpen] = useState(false);
  const [filters, setFilters] = useState<SearchFiltersType>({});
  const [stats, setStats] = useState<Stats>({
    datasources: null,
    organizations: null,
    downloads: null,
  });

  // Fetch stats from backend
  useEffect(() => {
    const fetchStats = async () => {
      try {
        const response = await fetch('http://localhost:8000/stats');
        if (!response.ok) {
          throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        setStats({
          datasources: data.datasources,
          organizations: data.organizations,
          downloads: data.downloads,
        });
      } catch (error) {
        console.error('Failed to fetch stats:', error);
        // Keep stats as null to show "-" placeholders
      }
    };

    fetchStats();
  }, []);

  return (
    <div className="flex flex-col">
      {/* Hero Section with Search */}
      <section className="relative overflow-hidden">
        <GrainGradient variant="hero" className="absolute inset-0 -z-10" />
        <div className="container flex flex-col items-center justify-center gap-12 pb-20 pt-16 md:pb-28 md:pt-24 lg:pb-32 lg:pt-28 min-h-[75vh]">
          <div className="flex w-full max-w-7xl flex-col items-center gap-8 text-center">
            <h1 className="animate-slide-in text-4xl font-bold leading-tight tracking-tighter md:text-6xl lg:text-7xl lg:leading-[1.1] flex items-center gap-3">
              <Logo size="lg" linkable={false} className="text-4xl md:text-6xl lg:text-7xl" />
              <span>registry</span>
            </h1>
            <p className="max-w-[750px] text-lg text-muted-foreground sm:text-xl animate-fade-in">
              {t('hero.subtitle')}
            </p>

            {/* Search Bar */}
            <div className="w-full max-w-4xl animate-fade-in space-y-3">
              <SearchBar
                variant="hero"
                filters={filters}
                onFiltersOpen={() => setIsFiltersOpen(true)}
                placeholder={t('hero.search.placeholder')}
              />

              {/* Stats - Prominent */}
              <div className="flex items-center justify-center gap-10 text-base text-muted-foreground">
                <div className="flex items-center gap-2.5">
                  <Package className="h-5 w-5" />
                  <span className="font-semibold" style={{ textShadow: '0 1px 3px rgba(0,0,0,0.15)' }}>
                    {stats.datasources !== null
                      ? `${stats.datasources.toLocaleString()}+`
                      : '-'} {t('stats.packages')}
                  </span>
                </div>
                <div className="flex items-center gap-2.5">
                  <Building2 className="h-5 w-5" />
                  <span className="font-semibold" style={{ textShadow: '0 1px 3px rgba(0,0,0,0.15)' }}>
                    {stats.organizations !== null
                      ? `${stats.organizations.toLocaleString()}+`
                      : '-'} {t('stats.organizations')}
                  </span>
                </div>
                {stats.downloads !== null && stats.downloads > 100 && (
                  <div className="flex items-center gap-2.5">
                    <Download className="h-5 w-5" />
                    <span className="font-semibold" style={{ textShadow: '0 1px 3px rgba(0,0,0,0.15)' }}>
                      {stats.downloads.toLocaleString()}+ {t('stats.downloads')}
                    </span>
                  </div>
                )}
              </div>
            </div>

            {/* Getting Started */}
            <div className="w-full flex flex-col items-center gap-3 pt-12">
              <p className="text-sm text-muted-foreground/80 font-medium">
                Get your first dataset in 30 seconds
              </p>
              <GettingStarted />
              <p className="text-xs text-muted-foreground/70 text-center max-w-3xl px-4 pt-2">
                Then integrate in Nextflow/Snakemake, generate citations & data availability statements, or run post-pull scripts to process your data
              </p>
            </div>

          </div>
        </div>
      </section>

      {/* Features Section */}
      <section className="border-t bg-muted/30 py-16 md:py-24 lg:py-32">
        <div className="container">
          <div className="mx-auto flex max-w-[980px] flex-col items-center gap-4 text-center">
            <h2 className="text-3xl font-bold leading-tight tracking-tighter md:text-5xl">
              {t('features.title')}
            </h2>
            <p className="max-w-[750px] text-lg text-muted-foreground">
              {t('features.subtitle')}
            </p>
          </div>

          <div className="mx-auto grid max-w-6xl auto-rows-fr grid-cols-1 gap-6 py-12 sm:grid-cols-2 lg:grid-cols-4">
            <FeatureCard
              icon={<Lock className="h-10 w-10" />}
              title={t('features.reproducibility.title')}
              description={t('features.reproducibility.description')}
            />
            <FeatureCard
              icon={<Handshake className="h-10 w-10" />}
              title={t('features.collaboration.title')}
              description={t('features.collaboration.description')}
            />
            <FeatureCard
              icon={<HardDrive className="h-10 w-10" />}
              title={t('features.resourceManagement.title')}
              description={t('features.resourceManagement.description')}
            />
            <FeatureCard
              icon={<BookText className="h-10 w-10" />}
              title={t('features.citation.title')}
              description={t('features.citation.description')}
            />
            <FeatureCard
              icon={<Shield className="h-10 w-10" />}
              title={t('features.integrity.title')}
              description={t('features.integrity.description')}
            />
            <FeatureCard
              icon={<SearchIcon className="h-10 w-10" />}
              title={t('features.discovery.title')}
              description={t('features.discovery.description')}
            />
            <FeatureCard
              icon={<Zap className="h-10 w-10" />}
              title={t('features.workflow.title')}
              description={t('features.workflow.description')}
            />

            {/* Open Source CTA Card - Last in Grid */}
            <div className="group relative flex h-full flex-col gap-4 rounded-lg border-2 border-primary bg-gradient-to-br from-primary/10 via-primary/5 to-card/50 p-6 backdrop-blur-sm transition-all duration-300 hover:scale-105 hover:border-primary hover:from-primary/20">
              <div className="grain-visible absolute inset-0 -z-10 opacity-20" />
              <div className="text-primary transition-transform duration-300 group-hover:scale-110">
                <Github className="h-10 w-10" />
              </div>
              <div className="space-y-2">
                <h3 className="text-xl font-bold">{t('features.openSource.title')}</h3>
                <p className="text-sm text-muted-foreground">{t('features.openSource.description')}</p>
              </div>
              <div className="mt-auto flex justify-end">
                <a
                  href="https://github.com/datadir-lab/bdp"
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  <Button size="sm" className="gap-2">
                    <Star className="h-4 w-4" />
                    {t('features.openSource.button')}
                  </Button>
                </a>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Search Filters Dialog */}
      <SearchFilters
        open={isFiltersOpen}
        onOpenChange={setIsFiltersOpen}
        filters={filters}
        onFiltersChange={setFilters}
      />

    </div>
  );
}

function FeatureCard({
  icon,
  title,
  description,
}: {
  icon: React.ReactNode;
  title: string;
  description: string;
}) {
  return (
    <div className="group relative flex h-full flex-col gap-4 rounded-lg border bg-card/50 p-6 backdrop-blur-sm transition-all duration-300 hover:scale-105 hover:border-primary/50 hover:bg-card">
      <div className="text-primary transition-transform duration-300 group-hover:scale-110">{icon}</div>
      <div className="space-y-2">
        <h3 className="text-xl font-bold">{title}</h3>
        <p className="text-sm text-muted-foreground">{description}</p>
      </div>
    </div>
  );
}
