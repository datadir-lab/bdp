'use client';

import * as React from 'react';
import Image from 'next/image';
import { Activity, Clock } from 'lucide-react';
import { JobStatusBadge } from './job-status-badge';
import { formatRelativeTime } from '@/lib/utils';
import type { Job } from '@/lib/types/job';

export interface TimelineViewProps {
  jobs: Array<
    Job & {
      organization?: {
        name: string;
        logo_url: string | null;
      };
    }
  >;
  className?: string;
}

export function TimelineView({ jobs, className }: TimelineViewProps) {
  const formatDuration = (job: Job): string | null => {
    if (!job.completed_at || !job.started_at) return null;

    const duration = Math.floor(
      (new Date(job.completed_at).getTime() - new Date(job.started_at).getTime()) / 1000
    );

    const hours = Math.floor(duration / 3600);
    const minutes = Math.floor((duration % 3600) / 60);
    const seconds = duration % 60;

    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    } else if (minutes > 0) {
      return `${minutes}m ${seconds}s`;
    } else {
      return `${seconds}s`;
    }
  };

  if (jobs.length === 0) {
    return (
      <div className="text-center py-12">
        <Activity className="h-12 w-12 mx-auto mb-3 text-muted-foreground opacity-50" />
        <p className="text-muted-foreground">No jobs found</p>
      </div>
    );
  }

  return (
    <div className={className}>
      <div className="space-y-3">
        {jobs.map((job) => {
          const duration = formatDuration(job);

          return (
            <div
              key={job.id}
              className="flex items-center gap-4 p-4 rounded-lg border bg-card hover:border-primary transition-colors"
            >
              {/* Organization Logo */}
              <div className="flex-shrink-0">
                {job.organization?.logo_url ? (
                  <Image
                    src={job.organization.logo_url}
                    alt={`${job.organization.name} logo`}
                    width={32}
                    height={32}
                    className="rounded"
                  />
                ) : (
                  <div className="w-8 h-8 rounded bg-muted flex items-center justify-center">
                    <Activity className="h-4 w-4 text-muted-foreground" />
                  </div>
                )}
              </div>

              {/* Job Details */}
              <div className="flex-1 min-w-0 grid grid-cols-1 md:grid-cols-4 gap-2 md:gap-4">
                <div className="min-w-0">
                  <p className="text-sm font-medium truncate">
                    {job.organization?.name || 'Unknown'}
                  </p>
                  <p className="text-xs text-muted-foreground truncate">
                    {job.job_type}
                  </p>
                </div>

                <div className="flex items-center">
                  <JobStatusBadge status={job.status} size="sm" />
                </div>

                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <Clock className="h-3 w-3 flex-shrink-0" aria-hidden="true" />
                  <span className="truncate">
                    {duration ? `Duration: ${duration}` : 'In progress'}
                  </span>
                </div>

                <div className="text-xs text-muted-foreground">
                  {formatRelativeTime(job.started_at || job.created_at)}
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
