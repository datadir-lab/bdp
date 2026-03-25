'use client';

import { useState, useRef } from 'react';
import { fetchSemanticSearch } from '@/lib/vectors/tile-loader';

interface Props {
  onResult: (x: number, y: number) => void;
}

export default function VectorSearchBar({ onResult }: Props) {
  const [query, setQuery] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleSearch = async (q: string) => {
    if (!q.trim()) return;
    setLoading(true);
    setError(null);
    try {
      const results = await fetchSemanticSearch(q, 20);
      // Fly to centroid of top results that have coordinates
      const withCoords = results.filter(r => r.x != null && r.y != null);
      if (withCoords.length > 0) {
        const cx = withCoords.reduce((s, r) => s + (r.x ?? 0), 0) / withCoords.length;
        const cy = withCoords.reduce((s, r) => s + (r.y ?? 0), 0) / withCoords.length;
        onResult(cx, cy);
      } else {
        setError('No results with known coordinates.');
      }
    } catch {
      setError('Search failed.');
    } finally {
      setLoading(false);
    }
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    setQuery(val);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => handleSearch(val), 300);
  };

  return (
    <div className="absolute top-10 left-1/2 -translate-x-1/2 z-10 w-80">
      <input
        type="text"
        value={query}
        onChange={handleChange}
        placeholder="Search the embedding space…"
        className="w-full px-3 py-2 text-sm rounded-lg border bg-background/90 backdrop-blur shadow-sm focus:outline-none focus:ring-2 focus:ring-primary"
      />
      {loading && <div className="text-xs text-muted-foreground mt-1 text-center">Searching…</div>}
      {error && <div className="text-xs text-destructive mt-1 text-center">{error}</div>}
    </div>
  );
}
