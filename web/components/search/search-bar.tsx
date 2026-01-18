'use client';

import * as React from 'react';
import { useRouter, usePathname } from '@/i18n/navigation';
import { Search, Filter, X, Loader2 } from 'lucide-react';
import { useTranslations } from 'next-intl';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandItem,
  CommandList,
} from '@/components/ui/command';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
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
  const [suggestions, setSuggestions] = React.useState<SearchSuggestion[]>([]);
  const [isLoading, setIsLoading] = React.useState(false);
  const debouncedQuery = useDebounce(query, 300);
  const inputRef = React.useRef<HTMLInputElement>(null);

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
        return;
      }

      setIsLoading(true);
      try {
        const data = await getSuggestions({
          q: debouncedQuery,
          limit: 10,
          type_filter: filters?.types,
        });
        setSuggestions(data);
      } catch (error) {
        console.error('Failed to fetch suggestions:', error);
        setSuggestions([]);
      } finally {
        setIsLoading(false);
      }
    };

    fetchSuggestions();
  }, [debouncedQuery, filters]);

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
      let url: string;
      if (suggestion.entry_type === 'organization') {
        url = `/organizations/${suggestion.slug}`;
      } else {
        url = `/sources/${suggestion.organization_slug}/${suggestion.slug}`;
      }
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
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        handleSearch();
      }
      if (e.key === 'Escape') {
        setIsOpen(false);
      }
    },
    [handleSearch]
  );

  const getTypeIcon = (entry_type: SearchSuggestion['entry_type']) => {
    const icons = {
      data_source: '🗄️',
      tool: '🛠️',
      organization: '🏢',
    };
    return icons[entry_type] || '📋';
  };

  return (
    <div className={cn('relative w-full', className)}>
      <Popover open={isOpen && suggestions.length > 0} onOpenChange={setIsOpen}>
        <form onSubmit={handleSearch} className="relative w-full">
          <div
            className={cn(
              'relative flex items-center gap-2 rounded-lg border bg-background transition-all',
              'focus-within:ring-2 focus-within:ring-ring focus-within:ring-offset-2',
              isHero
                ? 'h-14 px-5 shadow-lg hover:shadow-xl'
                : 'h-10 px-3 shadow-sm hover:shadow-md'
            )}
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
              onChange={(e) => {
                setQuery(e.target.value);
                setIsOpen(true);
              }}
              onKeyDown={handleKeyDown}
              onFocus={() => setIsOpen(true)}
              placeholder={placeholder || t('placeholder')}
              className={cn(
                'flex-1 bg-transparent outline-none placeholder:text-muted-foreground',
                'focus:outline-none focus:ring-0 focus:border-none focus-visible:outline-none',
                'border-0 focus:border-0 active:border-0',
                isHero ? 'text-base' : 'text-sm'
              )}
              style={{ outline: 'none', boxShadow: 'none', border: 'none' }}
            />

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
                  setIsOpen(false);
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

        {/* Autocomplete Dropdown */}
        <PopoverContent
          className="w-[var(--radix-popover-trigger-width)] p-0"
          align="start"
          side="bottom"
          sideOffset={8}
          onOpenAutoFocus={(e) => e.preventDefault()}
        >
          <Command>
            <CommandList>
              <CommandEmpty className="py-6 text-center text-sm text-muted-foreground">
                {t('noResults')}
              </CommandEmpty>
              <CommandGroup>
                {suggestions.map((suggestion) => (
                  <CommandItem
                    key={suggestion.id}
                    value={suggestion.id}
                    onSelect={() => handleSuggestionClick(suggestion)}
                    className="cursor-pointer"
                  >
                    <span className="mr-2 text-lg">{getTypeIcon(suggestion.entry_type)}</span>
                    <div className="flex flex-1 flex-col gap-1">
                      <span className="font-medium">{suggestion.name}</span>
                      {suggestion.latest_version && (
                        <span className="text-xs text-muted-foreground">
                          v{suggestion.latest_version}
                        </span>
                      )}
                    </div>
                    <Badge variant="outline" className="ml-2 text-xs capitalize">
                      {suggestion.entry_type.replace('_', ' ')}
                    </Badge>
                  </CommandItem>
                ))}
              </CommandGroup>
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>
    </div>
  );
}
