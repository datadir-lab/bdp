'use client';

import * as React from 'react';
import { useRouter, usePathname } from '@/i18n/navigation';
import { createPortal } from 'react-dom';
import {
  Search,
  Filter,
  X,
  Loader2,
  Building2,
  Database,
  Wrench,
  Dna,
  FileText,
  Workflow,
  Activity,
  Box,
  Binary,
  Hexagon,
  FileCode,
} from 'lucide-react';
import { useTranslations } from 'next-intl';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Popover, PopoverContent, PopoverAnchor } from '@/components/ui/popover';
import { Badge } from '@/components/ui/badge';
import { useDebounce } from '@/hooks/use-debounce';
import { SearchFilters as SearchFiltersType, SearchSuggestion } from '@/lib/types/search';
import { getSuggestions } from '@/lib/api/search';

interface SearchBarProps {
  className?: string;
  variant?: 'hero' | 'header';
  placeholder?: string;
  onFiltersOpen?: () => void;
  filters?: SearchFiltersType;
}

export function SearchBar({
  className,
  variant = 'hero',
  placeholder,
  onFiltersOpen,
  filters,
}: SearchBarProps) {
  const t = useTranslations('search');
  const router = useRouter();
  const pathname = usePathname();
  const [query, setQuery] = React.useState('');
  const [isOpen, setIsOpen] = React.useState(false);
  const [isFocused, setIsFocused] = React.useState(false);
  const [suggestions, setSuggestions] = React.useState<SearchSuggestion[]>([]);
  const [isLoading, setIsLoading] = React.useState(false);
  const [manuallyClosedFor, setManuallyClosedFor] = React.useState('');
  const [selectedIndex, setSelectedIndex] = React.useState(0);
  const debouncedQuery = useDebounce(query, 300);
  const inputRef = React.useRef<HTMLInputElement>(null);
  const containerRef = React.useRef<HTMLDivElement>(null);
  const [mounted, setMounted] = React.useState(false);

  React.useEffect(() => {
    setMounted(true);
  }, []);

  const isHero = variant === 'hero';
  const filterCount = React.useMemo(() => {
    if (!filters) return 0;
    let count = 0;
    if (filters.types?.length) count += filters.types.length;
    if (filters.organizations?.length) count += filters.organizations.length;
    if (filters.tags?.length) count += filters.tags.length;
    if (filters.dateRange?.from || filters.dateRange?.to) count += 1;
    return count;
  }, [filters]);

  // Fetch autocomplete suggestions
  React.useEffect(() => {
    const fetchSuggestions = async () => {
      if (!debouncedQuery || debouncedQuery.length < 2) {
        setSuggestions([]);
        setIsLoading(false);
        return;
      }

      setIsLoading(true);
      try {
        const data = await getSuggestions({
          q: debouncedQuery,
          limit: 10,
          type_filter: filters?.types,
          source_type_filter: filters?.source_types,
        });

        // Filter out suggestions with missing required fields
        const validSuggestions = data.filter(suggestion => {
          // Check for valid slug
          if (!suggestion.slug || suggestion.slug === 'undefined' || suggestion.slug === '') {
            console.warn('Filtering out suggestion with invalid slug:', suggestion);
            return false;
          }

          // For non-organizations, check for valid organization_slug
          if (suggestion.entry_type !== 'organization' &&
              (!suggestion.organization_slug || suggestion.organization_slug === 'undefined' || suggestion.organization_slug === '')) {
            console.warn('Filtering out suggestion with invalid organization_slug:', suggestion);
            return false;
          }

          return true;
        });

        console.log(`Fetched ${data.length} suggestions, ${validSuggestions.length} valid`);
        setSuggestions(validSuggestions);
        setSelectedIndex(0); // Reset selection when new suggestions arrive
      } catch (error) {
        console.error('Failed to fetch suggestions:', error);
        setSuggestions([]);
      } finally {
        setIsLoading(false);
      }
    };

    fetchSuggestions();
  }, [debouncedQuery, filters]);

  // Auto-open dropdown when we have query and (loading or have results)
  // But respect manual close - don't reopen until query changes
  React.useEffect(() => {
    // Don't auto-open if not focused
    if (!isFocused) {
      setIsOpen(false);
      return;
    }

    if (query.length >= 2 && (isLoading || suggestions.length > 0)) {
      // Only auto-open if user hasn't manually closed for this query
      if (manuallyClosedFor !== query) {
        setIsOpen(true);
      }
    } else if (query.length < 2) {
      setIsOpen(false);
      setManuallyClosedFor('');
    }
  }, [query, isLoading, suggestions, manuallyClosedFor, isFocused]);

  // Reset manual close tracking when query changes
  React.useEffect(() => {
    if (query.length >= 2) {
      setManuallyClosedFor('');
    }
  }, [query]);

  const handleSearch = React.useCallback(
    (e?: React.FormEvent) => {
      e?.preventDefault();
      if (!query.trim()) return;

      // Build query params
      const params = new URLSearchParams();
      params.set('q', query.trim());

      if (filters?.types?.length) {
        params.set('types', filters.types.join(','));
      }
      if (filters?.source_types?.length) {
        params.set('source_types', filters.source_types.join(','));
      }
      if (filters?.organizations?.length) {
        params.set('organizations', filters.organizations.join(','));
      }
      if (filters?.tags?.length) {
        params.set('tags', filters.tags.join(','));
      }
      if (filters?.dateRange?.from) {
        params.set('from', filters.dateRange.from.toISOString());
      }
      if (filters?.dateRange?.to) {
        params.set('to', filters.dateRange.to.toISOString());
      }

      // Navigate to search results page
      router.push(`/search?${params.toString()}`);
      setIsOpen(false);
    },
    [query, filters, router]
  );

  const handleSuggestionClick = React.useCallback(
    (suggestion: SearchSuggestion) => {
      console.log('Suggestion clicked:', suggestion);

      // Skip if required fields are missing or are undefined/null
      if (!suggestion.slug || suggestion.slug === 'undefined') {
        console.error('Cannot navigate: missing or invalid slug', suggestion);
        return;
      }
      if (suggestion.entry_type !== 'organization' && (!suggestion.organization_slug || suggestion.organization_slug === 'undefined')) {
        console.error('Cannot navigate: missing or invalid organization_slug', suggestion);
        return;
      }

      let url: string;
      if (suggestion.entry_type === 'organization') {
        url = `/organizations/${suggestion.slug}`;
      } else {
        // Include version in URL to avoid redirect issues
        const version = suggestion.latest_version || 'latest';
        url = `/sources/${suggestion.organization_slug}/${suggestion.slug}/${version}`;
      }

      console.log('Navigating to:', url);
      router.push(url);
      setIsOpen(false);
      setQuery('');
    },
    [router]
  );

  const handleClear = React.useCallback(() => {
    setQuery('');
    setSuggestions([]);
    inputRef.current?.focus();
  }, []);

  const handleKeyDown = React.useCallback(
    (e: React.KeyboardEvent) => {
      // Handle keyboard navigation when popover is open with suggestions
      if (isOpen && suggestions.length > 0) {
        if (e.key === 'ArrowDown') {
          e.preventDefault();
          setSelectedIndex((prev) => Math.min(prev + 1, suggestions.length - 1));
          return;
        }
        if (e.key === 'ArrowUp') {
          e.preventDefault();
          setSelectedIndex((prev) => Math.max(prev - 1, 0));
          return;
        }
        if (e.key === 'Enter' && !e.shiftKey) {
          e.preventDefault();
          // Select the currently highlighted suggestion
          const selectedSuggestion = suggestions[selectedIndex];
          if (selectedSuggestion) {
            handleSuggestionClick(selectedSuggestion);
          }
          return;
        }
      }

      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        handleSearch();
      }
      if (e.key === 'Escape') {
        setIsOpen(false);
        setManuallyClosedFor(query);
      }
    },
    [handleSearch, query, isOpen, suggestions, selectedIndex, handleSuggestionClick]
  );

  const handleClose = React.useCallback(() => {
    inputRef.current?.blur();
    setIsOpen(false);
    setIsFocused(false);
    setManuallyClosedFor(query);
  }, [query, isOpen, isFocused]);

  const handleFocus = React.useCallback(() => {
    setIsFocused(true);
    setManuallyClosedFor('');
  }, []);

  // Click outside handler
  React.useEffect(() => {
    if (!isFocused && !isOpen) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        handleClose();
      }
    };

    // Use capture phase to catch events before they bubble
    document.addEventListener('mousedown', handleClickOutside, true);
    return () => document.removeEventListener('mousedown', handleClickOutside, true);
  }, [isFocused, isOpen, handleClose]);

  // Global keyboard shortcut for Ctrl+K
  React.useEffect(() => {
    const handleGlobalKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === 'k') {
        e.preventDefault();
        inputRef.current?.focus();
      }
    };

    document.addEventListener('keydown', handleGlobalKeyDown);
    return () => document.removeEventListener('keydown', handleGlobalKeyDown);
  }, []);

  const getIcon = (entry_type: SearchSuggestion['entry_type'], source_type?: string | null) => {
    const iconClass = "h-4 w-4 shrink-0 text-muted-foreground";

    // If it's an organization
    if (entry_type === 'organization') {
      return <Building2 className={iconClass} />;
    }

    // If it's a tool
    if (entry_type === 'tool') {
      return <Wrench className={iconClass} />;
    }

    // For data sources, use source_type to determine icon
    switch (source_type) {
      case 'protein':
        return <Dna className={iconClass} />;
      case 'genome':
        return <Binary className={iconClass} />;
      case 'organism':
      case 'taxonomy':
        return <Activity className={iconClass} />;
      case 'transcript':
        return <FileCode className={iconClass} />;
      case 'annotation':
        return <FileText className={iconClass} />;
      case 'structure':
        return <Hexagon className={iconClass} />;
      case 'pathway':
        return <Workflow className={iconClass} />;
      case 'bundle':
        return <Box className={iconClass} />;
      default:
        return <Database className={iconClass} />;
    }
  };

  const showBackdrop = isFocused || (isOpen && query.length >= 2);
  const popoverOpen = isOpen && query.length >= 2;

  return (
    <>
      {/* Backdrop - Rendered in portal to avoid layout shifts */}
      {mounted && showBackdrop && createPortal(
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-[60] animate-in fade-in duration-200 pointer-events-none" />,
        document.body
      )}

      <div
        ref={containerRef}
        className={cn('relative w-full', className)}
      >
        <Popover
          open={popoverOpen}
          modal={false}
          onOpenChange={(open) => {
            if (!open) {
              handleClose();
            }
          }}
        >
          <PopoverAnchor asChild>
            <form onSubmit={handleSearch} className="relative w-full z-[70]">
            <div
              className={cn(
                'relative flex items-center gap-2 rounded-lg border bg-background',
                isHero ? 'h-14 px-5' : 'h-10 px-3',
                isHero ? 'shadow-lg' : 'shadow-sm'
              )}
              style={{
                boxShadow: isFocused
                  ? '0 0 0 2px hsl(var(--ring)), 0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1)'
                  : undefined
              }}
            >
            {/* Search Icon */}
            <Search
              className={cn(
                'shrink-0 text-muted-foreground',
                isHero ? 'h-5 w-5' : 'h-4 w-4'
              )}
            />

            {/* Input */}
            <input
              ref={inputRef}
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={handleKeyDown}
              onFocus={handleFocus}
              placeholder={placeholder || t('placeholder')}
              className={cn(
                'flex-1 bg-transparent outline-none placeholder:text-muted-foreground',
                'focus:outline-none focus:ring-0 focus:border-none focus-visible:outline-none',
                'border-0 focus:border-0 active:border-0',
                isHero ? 'text-base' : 'text-sm'
              )}
              style={{ outline: 'none', boxShadow: 'none', border: 'none' }}
            />

            {/* Keyboard Hint - Only show when empty */}
            {!query && !isLoading && (
              <kbd className="hidden sm:inline-flex items-center gap-0.5 px-1.5 py-0.5 text-xs font-mono bg-muted rounded shrink-0">
                <span className="text-xs">Ctrl</span>+K
              </kbd>
            )}

            {/* Loading Spinner */}
            {isLoading && (
              <Loader2 className="h-4 w-4 shrink-0 animate-spin text-muted-foreground" />
            )}

            {/* Clear Button */}
            {query && !isLoading && (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={handleClear}
                className={cn('h-6 w-6 shrink-0 rounded-full p-0 hover:bg-muted')}
              >
                <X className="h-3 w-3" />
                <span className="sr-only">{t('clear')}</span>
              </Button>
            )}

            {/* Filter Button */}
            <div className="flex shrink-0 items-center gap-2">
              {filterCount > 0 && (
                <Badge variant="secondary" className="h-5 px-1.5 text-xs">
                  {filterCount}
                </Badge>
              )}
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => {
                  onFiltersOpen?.();
                }}
                className={cn(
                  'shrink-0 rounded-md p-0 hover:bg-muted',
                  isHero ? 'h-8 w-8' : 'h-7 w-7'
                )}
              >
                <Filter
                  className={cn(
                    'text-muted-foreground',
                    isHero ? 'h-4 w-4' : 'h-3.5 w-3.5'
                  )}
                />
                <span className="sr-only">{t('filtersButton')}</span>
              </Button>
            </div>

            {/* Search Button - Only in hero variant */}
            {isHero && (
              <Button
                type="submit"
                size="sm"
                className="h-9 shrink-0 px-4 font-medium"
              >
                {t('search')}
              </Button>
            )}
          </div>
        </form>
        </PopoverAnchor>

        {/* Autocomplete Dropdown */}
        {popoverOpen && (
          <PopoverContent
            className="w-[var(--radix-popover-trigger-width)] p-0 z-[100]"
            align="start"
            side="bottom"
            sideOffset={8}
            onOpenAutoFocus={(e) => e.preventDefault()}
          >
          <div className="max-h-[400px] overflow-y-auto">
            {isLoading && (
              <div className="py-6 text-center text-sm text-muted-foreground">
                <Loader2 className="h-4 w-4 animate-spin mx-auto mb-2" />
                <span>Searching...</span>
              </div>
            )}
            {!isLoading && suggestions.length === 0 && query.length > 0 && (
              <div className="py-6 text-center text-sm text-muted-foreground">
                {t('noResults')}
              </div>
            )}
            {!isLoading && suggestions.length > 0 && (
              <div className="p-1">
                {suggestions.map((suggestion, index) => (
                <div
                  key={suggestion.id}
                  onClick={() => handleSuggestionClick(suggestion)}
                  onMouseEnter={() => setSelectedIndex(index)}
                  className={cn(
                    "flex items-start gap-2 cursor-pointer py-2.5 px-2 rounded-md transition-colors",
                    index === selectedIndex ? "bg-accent" : "hover:bg-accent/50"
                  )}
                >
                  <div className="mr-0.5 shrink-0">
                    {getIcon(suggestion.entry_type, suggestion.source_type)}
                  </div>
                  <div className="flex flex-1 flex-col gap-0.5 min-w-0">
                    <div className="flex items-baseline gap-2 truncate">
                      <span className="text-xs font-mono text-muted-foreground shrink-0">
                        {suggestion.slug}
                      </span>
                      <span className="font-medium text-sm leading-tight truncate">
                        {suggestion.name}
                      </span>
                    </div>
                    <div className="flex items-center gap-1.5 text-xs text-muted-foreground leading-tight">
                      {suggestion.entry_type !== 'organization' && suggestion.organization_slug && (
                        <>
                          <span className="truncate">
                            {suggestion.organization_slug.replace(/-/g, ' ').replace(/\b\w/g, l => l.toUpperCase())}
                          </span>
                          <span className="shrink-0">•</span>
                        </>
                      )}
                      {suggestion.latest_version && (
                        <>
                          <span className="shrink-0">v{suggestion.latest_version}</span>
                          <span className="shrink-0">•</span>
                        </>
                      )}
                      <span className="capitalize shrink-0">
                        {suggestion.entry_type.replace('_', ' ')}
                      </span>
                      {suggestion.source_type && (
                        <>
                          <span className="shrink-0">•</span>
                          <span className="truncate capitalize">
                            {suggestion.source_type}
                          </span>
                        </>
                      )}
                    </div>
                  </div>
                </div>
                ))}
              </div>
            )}
          </div>

          {/* Keyboard Shortcuts Footer */}
          {!isLoading && (suggestions.length > 0 || query.length > 0) && (
            <div className="border-t px-3 py-2 flex items-center gap-4 text-xs text-muted-foreground">
              <span className="flex items-center gap-1">
                <kbd className="px-1.5 py-0.5 bg-muted rounded font-mono">↑</kbd>
                <kbd className="px-1.5 py-0.5 bg-muted rounded font-mono">↓</kbd>
                <span>navigate</span>
              </span>
              <span className="flex items-center gap-1">
                <kbd className="px-1.5 py-0.5 bg-muted rounded font-mono">↵</kbd>
                <span>select</span>
              </span>
              <span className="flex items-center gap-1">
                <kbd className="px-1.5 py-0.5 bg-muted rounded font-mono">esc</kbd>
                <span>close</span>
              </span>
            </div>
          )}
        </PopoverContent>
        )}
      </Popover>
      </div>
    </>
  );
}
