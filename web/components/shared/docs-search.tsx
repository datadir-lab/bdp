'use client';

import { useState, useEffect, useCallback, useRef } from 'react';
import { useTranslations } from 'next-intl';
import { Search, X } from 'lucide-react';

interface PagefindResult {
  id: string;
  data: () => Promise<{
    url: string;
    content: string;
    excerpt: string;
    meta: {
      title: string;
      description?: string;
    };
  }>;
}

interface PagefindSearch {
  search: (query: string) => Promise<{ results: PagefindResult[] }>;
}

interface SearchResult {
  id: string;
  title: string;
  url: string;
  excerpt: string;
}

declare global {
  interface Window {
    pagefind?: PagefindSearch;
  }
}

export function DocsSearch() {
  const t = useTranslations('docsSearch');
  const [isOpen, setIsOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResult[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [pagefind, setPagefind] = useState<PagefindSearch | null>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const debounceTimerRef = useRef<NodeJS.Timeout | undefined>(undefined);

  // Load Pagefind on mount
  useEffect(() => {
    const loadPagefind = async () => {
      if (typeof window === 'undefined') return;

      // Check if already loaded
      if (window.pagefind) {
        setPagefind(window.pagefind as PagefindSearch);
        console.log('Pagefind already loaded');
        return;
      }

      try {
        console.log('Loading Pagefind...');

        // Load Pagefind loader script (which imports the module and attaches to window)
        const script = document.createElement('script');
        script.src = '/pagefind-loader.js';
        script.type = 'module';

        document.head.appendChild(script);

        // Poll for window.pagefind to become available
        let attempts = 0;
        const maxAttempts = 30; // 3 seconds total

        while (attempts < maxAttempts) {
          await new Promise(resolve => setTimeout(resolve, 100));

          if (window.pagefind) {
            console.log('Pagefind initialized successfully');
            setPagefind(window.pagefind as PagefindSearch);
            return;
          }

          attempts++;
        }

        console.error('Pagefind not available after timeout');
      } catch (error) {
        console.error('Error loading Pagefind:', error);
      }
    };

    loadPagefind();
  }, []);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Cmd/Ctrl + Shift + K to open search
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === 'K') {
        e.preventDefault();
        setIsOpen(true);
      }
      // Escape to close
      if (e.key === 'Escape') {
        setIsOpen(false);
        setQuery('');
        setResults([]);
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, []);

  // Focus input when modal opens
  useEffect(() => {
    if (isOpen && searchInputRef.current) {
      searchInputRef.current.focus();
    }
  }, [isOpen]);

  // Debounced search
  const performSearch = useCallback(
    async (searchQuery: string) => {
      if (!pagefind || !searchQuery.trim()) {
        setResults([]);
        setIsLoading(false);
        return;
      }

      setIsLoading(true);
      try {
        const searchResults = await pagefind.search(searchQuery);
        const processedResults = await Promise.all(
          searchResults.results.slice(0, 5).map(async (result) => {
            const data = await result.data();

            // Transform URL: remove locale prefix and .html extension
            let url = data.url;

            // Remove leading slash
            if (url.startsWith('/')) {
              url = url.substring(1);
            }

            // Remove locale prefix (en/, de/, etc.)
            url = url.replace(/^(en|de)\//, '');

            // Remove .html extension
            url = url.replace(/\.html$/, '');

            // Add leading slash back
            url = `/${url}`;

            return {
              id: result.id,
              title: data.meta.title || 'Untitled',
              url: url,
              excerpt: data.excerpt || data.content.slice(0, 150) + '...',
            };
          })
        );
        setResults(processedResults);
        setSelectedIndex(0);
      } catch (error) {
        console.error('Search error:', error);
        setResults([]);
      } finally {
        setIsLoading(false);
      }
    },
    [pagefind]
  );

  // Handle search input change with debounce
  const handleSearchChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setQuery(value);

    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current);
    }

    debounceTimerRef.current = setTimeout(() => {
      performSearch(value);
    }, 200);
  };

  // Keyboard navigation in results
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((prev) => Math.min(prev + 1, results.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((prev) => Math.max(prev - 1, 0));
    } else if (e.key === 'Enter' && results[selectedIndex]) {
      window.location.href = results[selectedIndex].url;
      setIsOpen(false);
    }
  };

  return (
    <>
      {/* Search Trigger Button */}
      <button
        onClick={() => setIsOpen(true)}
        className="flex items-center gap-2 px-3 py-1.5 text-sm text-muted-foreground border rounded-md hover:bg-accent transition-colors"
        title={pagefind ? 'Search documentation' : 'Search loading...'}
      >
        <Search className="h-4 w-4" />
        <span className="hidden sm:inline">{t('buttonText')}</span>
        <kbd className="hidden sm:inline-flex items-center gap-0.5 px-1.5 py-0.5 text-xs font-mono bg-muted rounded">
          <span className="text-xs">⌘</span>+<span className="text-xs">Shift</span>+K
        </kbd>
      </button>

      {/* Search Modal */}
      {isOpen && (
        <div
          className="fixed inset-0 z-50 bg-background/80 backdrop-blur-sm"
          onClick={() => setIsOpen(false)}
        >
          <div
            className="fixed left-[50%] top-[20%] translate-x-[-50%] w-full max-w-2xl bg-background border rounded-lg shadow-lg"
            onClick={(e) => e.stopPropagation()}
          >
            {/* Search Input */}
            <div className="flex items-center gap-3 border-b px-4 py-3">
              <Search className="h-5 w-5 text-muted-foreground" />
              <input
                ref={searchInputRef}
                type="text"
                value={query}
                onChange={handleSearchChange}
                onKeyDown={handleKeyDown}
                placeholder={t('placeholder')}
                className="flex-1 bg-transparent outline-none border-none focus:outline-none focus:ring-0 text-sm"
              />
              {isLoading && (
                <div className="animate-spin h-4 w-4 border-2 border-primary border-t-transparent rounded-full" />
              )}
              <button
                onClick={() => setIsOpen(false)}
                className="text-muted-foreground hover:text-foreground"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            {/* Search Results */}
            <div className="max-h-96 overflow-y-auto p-2">
              {!pagefind && (
                <div className="p-8 text-center text-sm text-muted-foreground">
                  Loading search index...
                </div>
              )}
              {pagefind && results.length === 0 && query && !isLoading && (
                <div className="p-8 text-center text-sm text-muted-foreground">
                  {t('noResults', { query })}
                </div>
              )}
              {pagefind && results.length === 0 && !query && (
                <div className="p-8 text-center text-sm text-muted-foreground">
                  {t('startTyping')}
                </div>
              )}
              {results.map((result, index) => (
                <a
                  key={result.id}
                  href={result.url}
                  className={`block p-3 rounded-md transition-colors ${
                    index === selectedIndex
                      ? 'bg-accent'
                      : 'hover:bg-accent/50'
                  }`}
                  onClick={() => setIsOpen(false)}
                >
                  <div className="font-medium text-sm mb-1">{result.title}</div>
                  <div
                    className="text-xs text-muted-foreground line-clamp-2"
                    dangerouslySetInnerHTML={{ __html: result.excerpt }}
                  />
                </a>
              ))}
            </div>

            {/* Footer */}
            <div className="border-t px-4 py-2 flex items-center justify-between text-xs text-muted-foreground">
              <div className="flex items-center gap-4">
                <span className="flex items-center gap-1">
                  <kbd className="px-1.5 py-0.5 bg-muted rounded">↑</kbd>
                  <kbd className="px-1.5 py-0.5 bg-muted rounded">↓</kbd>
                  navigate
                </span>
                <span className="flex items-center gap-1">
                  <kbd className="px-1.5 py-0.5 bg-muted rounded">↵</kbd>
                  select
                </span>
                <span className="flex items-center gap-1">
                  <kbd className="px-1.5 py-0.5 bg-muted rounded">esc</kbd>
                  close
                </span>
              </div>
              {pagefind && (
                <span className="text-xs">Powered by Pagefind</span>
              )}
            </div>
          </div>
        </div>
      )}
    </>
  );
}
