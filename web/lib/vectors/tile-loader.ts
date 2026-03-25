const API_BASE = '/api/v1/vectors';

export interface TilePoint {
  id:   string;
  x:    number;
  y:    number;
  l:    string;   // label
  et:   string;   // entry_type
  st:   string;   // source_type ('' if null)
  org:  string;
  slug: string;
}

export interface VectorStats {
  current_run_id:  string | null;
  status:          string | null;
  entry_count:     number | null;
  embedded_count:  number | null;
  projected_count: number | null;
  projected_at:    string | null;
  tile_prefix:     string | null;
}

// In-session tile cache — avoids re-fetching on pan-back
const tileCache = new Map<string, TilePoint[]>();

export async function fetchStats(): Promise<VectorStats> {
  const res = await fetch(`${API_BASE}/stats`);
  if (!res.ok) throw new Error(`Stats fetch failed: ${res.status}`);
  const json = await res.json();
  return json.data as VectorStats;
}

export async function fetchTile(
  runId: string,
  z: number,
  tx: number,
  ty: number,
): Promise<TilePoint[]> {
  const key = `${runId}/${z}/${tx}/${ty}`;
  if (tileCache.has(key)) return tileCache.get(key)!;

  const res = await fetch(`${API_BASE}/tiles/${runId}/${z}/${tx}/${ty}`);
  if (res.status === 404) {
    tileCache.set(key, []);
    return [];
  }
  if (!res.ok) throw new Error(`Tile fetch failed: ${res.status}`);

  const points: TilePoint[] = await res.json();
  tileCache.set(key, points);
  return points;
}

/** Fetch all tiles for the current viewport at a given zoom level. */
export async function fetchViewportTiles(
  runId: string,
  zoom: number,
  xMin: number, xMax: number,
  yMin: number, yMax: number,
  totalBounds: { x: [number, number]; y: [number, number] },
): Promise<TilePoint[]> {
  const nCells = Math.pow(2, zoom);
  const cellW = (totalBounds.x[1] - totalBounds.x[0]) / nCells;
  const cellH = (totalBounds.y[1] - totalBounds.y[0]) / nCells;

  const txMin = Math.max(0, Math.floor((xMin - totalBounds.x[0]) / cellW));
  const txMax = Math.min(nCells - 1, Math.floor((xMax - totalBounds.x[0]) / cellW));
  const tyMin = Math.max(0, Math.floor((yMin - totalBounds.y[0]) / cellH));
  const tyMax = Math.min(nCells - 1, Math.floor((yMax - totalBounds.y[0]) / cellH));

  const fetches: Promise<TilePoint[]>[] = [];
  for (let tx = txMin; tx <= txMax; tx++) {
    for (let ty = tyMin; ty <= tyMax; ty++) {
      fetches.push(fetchTile(runId, zoom, tx, ty));
    }
  }

  const results = await Promise.all(fetches);
  return results.flat();
}

export async function fetchSemanticSearch(
  q: string,
  k = 20,
): Promise<Array<{ slug: string; name: string; org_slug: string; x?: number; y?: number; similarity: number }>> {
  const res = await fetch(`${API_BASE}/search?q=${encodeURIComponent(q)}&k=${k}`);
  if (!res.ok) throw new Error(`Search failed: ${res.status}`);
  const json = await res.json();
  return json.data ?? [];
}

export async function fetchNeighbors(entryId: string, k = 6) {
  const res = await fetch(`${API_BASE}/${entryId}/neighbors?k=${k}`);
  if (!res.ok) return [];
  const json = await res.json();
  return json.data ?? [];
}
