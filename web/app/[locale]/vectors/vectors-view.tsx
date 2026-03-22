'use client';

import { useEffect, useRef, useState, useCallback } from 'react';
import createScatterplot from 'regl-scatterplot';
import {
  fetchStats, fetchViewportTiles, VectorStats, TilePoint
} from '@/lib/vectors/tile-loader';
import { getSourceTypeColor, SOURCE_TYPE_COLORS } from '@/lib/source-type-colors';
import VectorSidebar from './vector-sidebar';
import VectorSearchBar from './vector-search-bar';

const INITIAL_ZOOM = 3;
// Total projection space bounds (will be derived from first tile batch)
const DEFAULT_BOUNDS = { x: [-15, 15] as [number, number], y: [-15, 15] as [number, number] };

export default function VectorsView() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const scatterRef = useRef<ReturnType<typeof createScatterplot> | null>(null);
  const [stats, setStats] = useState<VectorStats | null>(null);
  const [points, setPoints] = useState<TilePoint[]>([]);
  const [selectedPoint, setSelectedPoint] = useState<TilePoint | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [enabledTypes, setEnabledTypes] = useState<Set<string>>(
    new Set(Object.keys(SOURCE_TYPE_COLORS))
  );

  // Load stats and initial tiles on mount
  useEffect(() => {
    (async () => {
      try {
        const s = await fetchStats();
        setStats(s);
        if (!s.current_run_id) { setLoading(false); return; }

        // Load initial viewport at zoom 3
        const initial = await fetchViewportTiles(
          s.current_run_id, INITIAL_ZOOM,
          DEFAULT_BOUNDS.x[0], DEFAULT_BOUNDS.x[1],
          DEFAULT_BOUNDS.y[0], DEFAULT_BOUNDS.y[1],
          DEFAULT_BOUNDS,
        );
        setPoints(initial);
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  // Initialize regl-scatterplot once canvas is ready
  useEffect(() => {
    if (!canvasRef.current || points.length === 0) return;

    const scatter = createScatterplot({
      canvas: canvasRef.current,
      pointSize: 3,
      opacity: 0.8,
      colorBy: 'category',
    });

    const data = points
      .filter(p => enabledTypes.has(p.st || 'other'))
      .map(p => [p.x, p.y, getSourceTypeColor(p.st)]);

    scatter.draw({ x: data.map(d => d[0] as number), y: data.map(d => d[1] as number) });

    scatter.subscribe('select', (data: unknown) => {
      const { points: selected } = data as { points: number[] };
      if (selected.length > 0) {
        setSelectedPoint(points[selected[0]] ?? null);
      }
    });

    scatterRef.current = scatter;
    return () => scatter.destroy();
  }, [points, enabledTypes]);

  const handleSearchResult = useCallback((x: number, y: number) => {
    scatterRef.current?.zoomToLocation([x, y], 0.5, { transition: true });
  }, []);

  if (loading) return (
    <div className="flex items-center justify-center h-screen text-muted-foreground">
      Loading vector space…
    </div>
  );

  if (error) return (
    <div className="flex items-center justify-center h-screen text-destructive">
      {error}
    </div>
  );

  if (!stats?.current_run_id) return (
    <div className="flex items-center justify-center h-screen text-muted-foreground">
      <div className="text-center">
        <p className="text-lg font-medium">No embeddings yet</p>
        <p className="text-sm mt-1">Run <code>bdp-embed embed</code> to get started.</p>
      </div>
    </div>
  );

  const embeddedPct = stats.embedded_count && stats.entry_count
    ? Math.round((stats.embedded_count / stats.entry_count) * 100)
    : 0;

  return (
    <div className="relative w-full h-screen overflow-hidden">
      {/* Stats bar */}
      <div className="absolute top-0 left-0 right-0 z-10 px-4 py-2 bg-background/80 backdrop-blur text-xs text-muted-foreground flex gap-4">
        <span>{stats.embedded_count?.toLocaleString()} of {stats.entry_count?.toLocaleString()} entries embedded ({embeddedPct}%)</span>
        {stats.projected_at && (
          <span>projected {new Date(stats.projected_at).toLocaleString()}</span>
        )}
        <span className="capitalize">{stats.status}</span>
      </div>

      {/* Search bar */}
      <VectorSearchBar onResult={handleSearchResult} />

      {/* Canvas */}
      <canvas ref={canvasRef} className="w-full h-full" />

      {/* Legend */}
      <div className="absolute bottom-4 left-4 z-10 flex flex-col gap-1">
        {Object.entries(SOURCE_TYPE_COLORS).map(([type, color]) => (
          <button
            key={type}
            onClick={() => setEnabledTypes(prev => {
              const next = new Set(prev);
              if (next.has(type)) next.delete(type); else next.add(type);
              return next;
            })}
            className={`flex items-center gap-1.5 text-xs px-2 py-0.5 rounded transition-opacity ${
              enabledTypes.has(type) ? 'opacity-100' : 'opacity-30'
            }`}
          >
            <span className="w-2 h-2 rounded-full" style={{ background: color }} />
            {type}
          </button>
        ))}
      </div>

      {/* Point count HUD */}
      <div className="absolute bottom-4 right-4 z-10 text-xs text-muted-foreground">
        {points.length.toLocaleString()} points visible
      </div>

      {/* Sidebar */}
      {selectedPoint && (
        <VectorSidebar
          point={selectedPoint}
          onClose={() => setSelectedPoint(null)}
        />
      )}
    </div>
  );
}
