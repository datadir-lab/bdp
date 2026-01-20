'use client';

import * as React from 'react';
import Image from 'next/image';
import { Activity, ChevronDown, ChevronUp } from 'lucide-react';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { JobCard } from './job-card';
import { JobStatusBadge } from './job-status-badge';
import { formatDate, formatNumber } from '@/lib/utils';
import type { OrganizationJobSummary } from '@/lib/types/job';

export interface OrganizationJobsSectionProps {
  summary: OrganizationJobSummary;
  className?: string;
}

export function OrganizationJobsSection({
  summary,
  className,
}: OrganizationJobsSectionProps) {
  const [showAll, setShowAll] = React.useState(false);
  const { organization, recent_jobs, sync_status, current_status } = summary;

  const displayedJobs = showAll ? recent_jobs : recent_jobs.slice(0, 5);
  const hasMoreJobs = recent_jobs.length > 5;

  const successRate =
    recent_jobs.length > 0
      ? (recent_jobs.filter((job) => job.status === 'Done').length /
          recent_jobs.length) *
        100
      : 0;

  return (
    <Card className={className}>
      <CardHeader>
        <div className="flex items-start justify-between gap-4">
          <div className="flex items-center gap-3 flex-1 min-w-0">
            {organization.logo_url ? (
              <Image
                src={organization.logo_url}
                alt={`${organization.name} logo`}
                width={40}
                height={40}
                className="rounded-md flex-shrink-0"
              />
            ) : (
              <div className="w-10 h-10 rounded-md bg-muted flex items-center justify-center flex-shrink-0">
                <Activity className="h-5 w-5 text-muted-foreground" />
              </div>
            )}
            <div className="flex-1 min-w-0">
              <CardTitle className="text-lg truncate">
                {organization.name}
              </CardTitle>
              {organization.description && (
                <CardDescription className="line-clamp-1 mt-1">
                  {organization.description}
                </CardDescription>
              )}
            </div>
          </div>
          <JobStatusBadge
            status={current_status === 'idle' ? 'Done' : current_status}
            size="sm"
          />
        </div>
      </CardHeader>

      <CardContent className="space-y-4">
        {sync_status && (
          <div className="space-y-3 pb-4 border-b">
            <div className="grid grid-cols-2 gap-3 text-sm">
              <div>
                <p className="text-muted-foreground text-xs">Last Sync</p>
                <p className="font-medium">
                  {sync_status.last_sync_at
                    ? formatDate(sync_status.last_sync_at)
                    : 'Never'}
                </p>
              </div>
              <div>
                <p className="text-muted-foreground text-xs">Version</p>
                <p className="font-medium truncate">
                  {sync_status.last_version || 'N/A'}
                </p>
              </div>
            </div>

            <div className="grid grid-cols-2 gap-3 text-sm">
              <div>
                <p className="text-muted-foreground text-xs">Total Entries</p>
                <p className="font-medium">
                  {formatNumber(sync_status.total_entries, 0)}
                </p>
              </div>
              <div>
                <p className="text-muted-foreground text-xs">Success Rate</p>
                <p className="font-medium">{successRate.toFixed(1)}%</p>
              </div>
            </div>
          </div>
        )}

        <div className="space-y-3">
          <h4 className="text-sm font-semibold">Recent Jobs</h4>
          {displayedJobs.length > 0 ? (
            <>
              <div className="space-y-2">
                {displayedJobs.map((job) => (
                  <JobCard key={job.id} job={job} />
                ))}
              </div>
              {hasMoreJobs && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setShowAll(!showAll)}
                  className="w-full"
                >
                  {showAll ? (
                    <>
                      <ChevronUp className="mr-2 h-4 w-4" />
                      Show Less
                    </>
                  ) : (
                    <>
                      <ChevronDown className="mr-2 h-4 w-4" />
                      Show More ({recent_jobs.length - 5} more)
                    </>
                  )}
                </Button>
              )}
            </>
          ) : (
            <div className="text-center py-8 text-muted-foreground">
              <Activity className="h-8 w-8 mx-auto mb-2 opacity-50" />
              <p className="text-sm">No recent jobs</p>
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
