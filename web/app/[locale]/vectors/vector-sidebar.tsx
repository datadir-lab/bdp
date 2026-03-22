'use client';

import { useEffect, useState } from 'react';
import { TilePoint, fetchNeighbors } from '@/lib/vectors/tile-loader';
import { getSourceTypeColor } from '@/lib/source-type-colors';

interface Props {
  point: TilePoint;
  onClose: () => void;
}

export default function VectorSidebar({ point, onClose }: Props) {
  const [neighbors, setNeighbors] = useState<TilePoint[]>([]);

  useEffect(() => {
    fetchNeighbors(point.id, 6).then(setNeighbors).catch(() => {});
  }, [point.id]);

  const color = getSourceTypeColor(point.st);
  const detailUrl = `/sources/${point.org}/${point.slug}`;

  return (
    <div className="absolute right-0 top-0 h-full w-72 bg-background/95 backdrop-blur border-l z-20 flex flex-col p-4 gap-3 overflow-y-auto">
      <div className="flex items-center justify-between">
        <span className="text-xs font-mono px-1.5 py-0.5 rounded" style={{ background: color + '33', color }}>
          {point.st || point.et}
        </span>
        <button onClick={onClose} className="text-muted-foreground hover:text-foreground text-lg leading-none">×</button>
      </div>

      <div className="font-medium text-sm leading-snug">{point.l}</div>

      <div className="text-xs text-muted-foreground">
        <span>{point.org}</span>
        <span className="mx-1">·</span>
        <span className="font-mono">{point.slug}</span>
      </div>

      <div className="text-xs text-muted-foreground font-mono">
        x: {point.x.toFixed(3)} · y: {point.y.toFixed(3)}
      </div>

      {neighbors.length > 0 && (
        <div>
          <div className="text-xs font-medium text-muted-foreground mb-1.5">Nearest in embedding space</div>
          <div className="flex flex-col gap-1">
            {neighbors.map((n: TilePoint) => (
              <a
                key={n.id}
                href={`/sources/${n.org}/${n.slug}`}
                className="flex items-center gap-1.5 text-xs hover:bg-muted rounded px-1 py-0.5 transition-colors"
              >
                <span className="w-1.5 h-1.5 rounded-full shrink-0" style={{ background: getSourceTypeColor(n.st) }} />
                <span className="truncate">{n.l}</span>
              </a>
            ))}
          </div>
        </div>
      )}

      <div className="mt-auto flex gap-2">
        <a href={detailUrl} className="flex-1 text-center text-xs py-1.5 px-2 bg-primary text-primary-foreground rounded hover:bg-primary/90 transition-colors">
          Open entry
        </a>
      </div>
    </div>
  );
}
