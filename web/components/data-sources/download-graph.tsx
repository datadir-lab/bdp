'use client';

import * as React from 'react';
import { Download, TrendingUp, TrendingDown } from 'lucide-react';
import { Area, AreaChart, CartesianGrid, XAxis, YAxis, Tooltip, ResponsiveContainer } from 'recharts';

interface DownloadGraphProps {
  downloadCount: number;
  totalDownloads?: number;
}

export function DownloadGraph({ downloadCount, totalDownloads }: DownloadGraphProps) {
  // Mock data for visualization - in a real app, this would come from an API
  const chartData = React.useMemo(() => {
    const data = [];
    const baseCount = Math.floor(downloadCount / 12);

    for (let i = 11; i >= 0; i--) {
      const month = new Date();
      month.setMonth(month.getMonth() - i);

      // Generate somewhat realistic download patterns
      const variance = Math.random() * 0.4 + 0.8; // 0.8 to 1.2
      const trend = (12 - i) / 12; // Slight upward trend
      const value = Math.floor(baseCount * variance * (1 + trend * 0.3));

      data.push({
        month: month.toLocaleDateString('en-US', { month: 'short' }),
        downloads: value,
      });
    }

    return data;
  }, [downloadCount]);

  const percentageChange = chartData.length >= 2 && chartData[chartData.length - 2].downloads > 0
    ? ((chartData[chartData.length - 1].downloads - chartData[chartData.length - 2].downloads) /
       chartData[chartData.length - 2].downloads * 100)
    : 0;

  const isPositive = percentageChange > 0;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Download className="h-4 w-4 text-muted-foreground" />
          <h3 className="font-semibold">Downloads</h3>
        </div>
        {percentageChange !== 0 && (
          <div className={`flex items-center gap-1 text-xs font-medium ${isPositive ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400'}`}>
            {isPositive ? (
              <TrendingUp className="h-3 w-3" />
            ) : (
              <TrendingDown className="h-3 w-3" />
            )}
            {isPositive ? '+' : ''}{percentageChange.toFixed(1)}%
          </div>
        )}
      </div>

      <div className="space-y-1">
        <div className="text-2xl font-bold">{downloadCount.toLocaleString()}</div>
        <div className="text-xs text-muted-foreground">
          {totalDownloads && totalDownloads > downloadCount ? (
            <span>{totalDownloads.toLocaleString()} total across all versions</span>
          ) : (
            <span>this version</span>
          )}
        </div>
      </div>

      {/* Area Chart */}
      <div className="h-[120px] w-full">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={chartData} margin={{ top: 5, right: 0, left: 0, bottom: 0 }}>
            <defs>
              <linearGradient id="downloadGradient" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="hsl(var(--primary))" stopOpacity={0.3} />
                <stop offset="95%" stopColor="hsl(var(--primary))" stopOpacity={0} />
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" className="stroke-muted" vertical={false} />
            <XAxis
              dataKey="month"
              tick={{ fontSize: 10 }}
              tickLine={false}
              axisLine={false}
              className="text-muted-foreground"
            />
            <Tooltip
              contentStyle={{
                backgroundColor: 'hsl(var(--card))',
                border: '1px solid hsl(var(--border))',
                borderRadius: '6px',
                fontSize: '12px',
              }}
              labelStyle={{ color: 'hsl(var(--foreground))' }}
              itemStyle={{ color: 'hsl(var(--primary))' }}
            />
            <Area
              type="monotone"
              dataKey="downloads"
              stroke="hsl(var(--primary))"
              strokeWidth={2}
              fill="url(#downloadGradient)"
              animationDuration={1000}
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
