'use client';

import * as React from 'react';
import { Clock, TrendingUp, AlertCircle, ChevronDown, ChevronUp } from 'lucide-react';
import { Card, CardContent } from '@/components/ui/card';
import { JobStatusBadge } from './job-status-badge';
import { cn, formatRelativeTime } from '@/lib/utils';
import type { Job } from '@/lib/types/job';

export interface JobCardProps {
  job: Job;
  className?: string;
}

export function JobCard({ job, className }: JobCardProps) {
  const startTime = job.started_at || job.created_at;
  const duration = job.completed_at && job.started_at
    ? Math.floor(
        (new Date(job.completed_at).getTime() - new Date(job.started_at).getTime()) / 1000
      )
    : null;

  const formatDuration = (seconds: number): string => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;

    if (hours > 0) {
      return `${hours}h ${minutes}m ${secs}s`;
    } else if (minutes > 0) {
      return `${minutes}m ${secs}s`;
    } else {
      return `${secs}s`;
    }
  };

  const progress = job.total_records
    ? Math.round((job.records_processed / job.total_records) * 100)
    : null;

  return (
    <Card
      className={cn(
        'transition-colors hover:border-primary',
        className
      )}
    >
      <CardContent className="p-4">
        <div className="flex items-start justify-between gap-2 mb-3">
          <div className="flex-1 min-w-0">
            <h4 className="text-sm font-medium truncate">{job.job_type}</h4>
            <p className="text-xs text-muted-foreground mt-0.5">
              {startTime ? `Started ${formatRelativeTime(startTime)}` : 'Not started'}
            </p>
          </div>
          <JobStatusBadge status={job.status} size="sm" />
        </div>

        <div className="space-y-2">
          {duration !== null && (
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <Clock className="h-3 w-3" aria-hidden="true" />
              <span>Duration: {formatDuration(duration)}</span>
            </div>
          )}

          {progress !== null && (
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <TrendingUp className="h-3 w-3" aria-hidden="true" />
              <span>
                Progress: {progress}% ({job.records_processed.toLocaleString()}/{job.total_records?.toLocaleString()})
              </span>
            </div>
          )}

          {job.records_stored > 0 && (
            <div className="text-xs text-muted-foreground">
              Stored: {job.records_stored.toLocaleString()} records
              {job.records_failed > 0 && (
                <span className="text-destructive ml-2">
                  Failed: {job.records_failed.toLocaleString()}
                </span>
              )}
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
