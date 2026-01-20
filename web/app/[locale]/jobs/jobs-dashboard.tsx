'use client';

import * as React from 'react';
import { RefreshCw, Loader2, AlertCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { OrganizationJobsSection } from '@/components/jobs/organization-jobs-section';
import { TimelineView } from '@/components/jobs/timeline-view';
import { listJobs, listSyncStatus } from '@/lib/api/jobs';
import { listOrganizations } from '@/lib/api/organizations';
import type { Job, SyncStatus, OrganizationJobSummary, JobStatus } from '@/lib/types/job';
import type { OrganizationListItem } from '@/lib/types/organization';

export function JobsDashboard() {
  const [jobs, setJobs] = React.useState<Job[]>([]);
  const [syncStatuses, setSyncStatuses] = React.useState<SyncStatus[]>([]);
  const [organizations, setOrganizations] = React.useState<OrganizationListItem[]>([]);
  const [isLoading, setIsLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);
  const [statusFilter, setStatusFilter] = React.useState<string>('all');
  const [autoRefresh, setAutoRefresh] = React.useState(true);
  const [activeView, setActiveView] = React.useState('organization');
  const [isRefreshing, setIsRefreshing] = React.useState(false);

  const fetchData = React.useCallback(async () => {
    try {
      setError(null);

      // Fetch organizations first (this should always work)
      const orgsRes = await listOrganizations({ limit: 100 });
      setOrganizations(orgsRes.data || []);

      // Try to fetch jobs and sync status (might not be available yet)
      try {
        const [jobsRes, syncRes] = await Promise.all([
          listJobs({ limit: 100 }),
          listSyncStatus(),
        ]);
        setJobs(jobsRes.jobs || []);
        setSyncStatuses(syncRes.statuses || []);
      } catch (jobsErr) {
        console.warn('Jobs API not available:', jobsErr);
        // Set empty arrays - UI will show "no jobs" state
        setJobs([]);
        setSyncStatuses([]);
        setError('Jobs API is not available. Please ensure the backend server is running.');
      }
    } catch (err) {
      console.error('Error fetching data:', err);
      setError(
        err instanceof Error
          ? err.message
          : 'Failed to fetch data. Please check if the backend server is running.'
      );
    } finally {
      setIsLoading(false);
      setIsRefreshing(false);
    }
  }, []);

  React.useEffect(() => {
    fetchData();
  }, [fetchData]);

  React.useEffect(() => {
    if (!autoRefresh) return;

    const interval = setInterval(() => {
      setIsRefreshing(true);
      fetchData();
    }, 30000); // 30 seconds

    return () => clearInterval(interval);
  }, [autoRefresh, fetchData]);

  const handleRefresh = () => {
    setIsRefreshing(true);
    fetchData();
  };

  const organizationSummaries = React.useMemo(() => {
    const summaries: OrganizationJobSummary[] = [];

    for (const org of organizations) {
      // Find jobs for this organization (match by job_type containing org name)
      const orgJobs = jobs
        .filter((job) =>
          job.job_type.toLowerCase().includes(org.name.toLowerCase())
        )
        .sort((a, b) => new Date(b.run_at).getTime() - new Date(a.run_at).getTime())
        .slice(0, 10);

      // Find sync status for this organization
      const syncStatus = syncStatuses.find((s) => s.organization_id === org.id);

      // Determine current status
      let currentStatus: JobStatus | 'idle' = 'idle';
      const runningJob = orgJobs.find((job) => job.status === 'Running');
      if (runningJob) {
        currentStatus = 'Running';
      } else if (orgJobs.length > 0) {
        currentStatus = orgJobs[0].status;
      }

      summaries.push({
        organization: {
          id: org.id,
          name: org.name,
          description: org.description || null,
          logo_url: org.logo_url || null,
          website_url: null,
        },
        recent_jobs: orgJobs,
        sync_status: syncStatus || null,
        current_status: currentStatus,
      });
    }

    // Only show organizations that have jobs OR sync status (have been synced at least once)
    return summaries.filter((s) => s.recent_jobs.length > 0 || s.sync_status !== null);
  }, [jobs, syncStatuses, organizations]);

  const filteredSummaries = React.useMemo(() => {
    if (statusFilter === 'all') return organizationSummaries;

    return organizationSummaries.filter((summary) => {
      const statusLower = statusFilter.toLowerCase();
      if (statusLower === 'running') {
        return summary.current_status === 'Running';
      } else if (statusLower === 'completed') {
        return summary.current_status === 'Done';
      } else if (statusLower === 'failed') {
        return summary.current_status === 'Failed';
      }
      return true;
    });
  }, [organizationSummaries, statusFilter]);

  const timelineJobs = React.useMemo(() => {
    const jobsWithOrg = jobs.map((job) => {
      const org = organizations.find((o) =>
        job.job_type.toLowerCase().includes(o.name.toLowerCase())
      );
      return {
        ...job,
        organization: org
          ? {
              name: org.name,
              logo_url: org.logo_url || null,
            }
          : undefined,
      };
    });

    // Sort by run_at descending
    return jobsWithOrg.sort(
      (a, b) => new Date(b.run_at).getTime() - new Date(a.run_at).getTime()
    );
  }, [jobs, organizations]);

  const filteredTimelineJobs = React.useMemo(() => {
    if (statusFilter === 'all') return timelineJobs;

    return timelineJobs.filter((job) => {
      const statusLower = statusFilter.toLowerCase();
      if (statusLower === 'running') return job.status === 'Running';
      if (statusLower === 'completed') return job.status === 'Done';
      if (statusLower === 'failed') return job.status === 'Failed';
      return true;
    });
  }, [timelineJobs, statusFilter]);

  if (isLoading) {
    return (
      <div className="flex flex-col items-center justify-center py-12">
        <Loader2 className="h-8 w-8 animate-spin text-primary mb-4" />
        <p className="text-muted-foreground">Loading jobs...</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold tracking-tight">Ingestion Jobs</h1>
            <p className="text-muted-foreground mt-1">
              Monitor data ingestion jobs across all organizations
            </p>
          </div>
          <Button
            onClick={handleRefresh}
            disabled={isRefreshing}
            variant="outline"
          >
            <RefreshCw
              className={`h-4 w-4 mr-2 ${isRefreshing ? 'animate-spin' : ''}`}
            />
            Refresh
          </Button>
        </div>
      </div>

      {/* Filters */}
      <div className="flex flex-wrap items-center gap-4">
        <div className="flex items-center gap-2">
          <Label htmlFor="status-filter">Status:</Label>
          <Select value={statusFilter} onValueChange={setStatusFilter}>
            <SelectTrigger id="status-filter" className="w-[150px]">
              <SelectValue placeholder="Filter by status" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All</SelectItem>
              <SelectItem value="running">Running</SelectItem>
              <SelectItem value="completed">Completed</SelectItem>
              <SelectItem value="failed">Failed</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div className="flex items-center gap-2">
          <Checkbox
            id="auto-refresh"
            checked={autoRefresh}
            onCheckedChange={(checked) => setAutoRefresh(checked === true)}
          />
          <Label htmlFor="auto-refresh" className="cursor-pointer">
            Auto-refresh (30s)
          </Label>
        </div>
      </div>

      {/* Error Display */}
      {error && (
        <div className={`rounded-lg border p-4 ${
          error.includes('Jobs API is not available')
            ? 'border-yellow-500 bg-yellow-50 dark:bg-yellow-950/20'
            : 'border-destructive bg-destructive/10'
        }`}>
          <div className="flex items-start gap-3">
            <AlertCircle className={`h-5 w-5 flex-shrink-0 mt-0.5 ${
              error.includes('Jobs API is not available')
                ? 'text-yellow-600 dark:text-yellow-500'
                : 'text-destructive'
            }`} />
            <div className="flex-1">
              <h3 className={`font-semibold mb-1 ${
                error.includes('Jobs API is not available')
                  ? 'text-yellow-800 dark:text-yellow-300'
                  : 'text-destructive'
              }`}>
                {error.includes('Jobs API is not available') ? 'Notice' : 'Error Loading Data'}
              </h3>
              <p className={`text-sm ${
                error.includes('Jobs API is not available')
                  ? 'text-yellow-700 dark:text-yellow-400'
                  : 'text-destructive/90'
              }`}>{error}</p>
            </div>
          </div>
        </div>
      )}

      {/* View Tabs */}
      <Tabs value={activeView} onValueChange={setActiveView}>
        <TabsList>
          <TabsTrigger value="organization">Organization Cards</TabsTrigger>
          <TabsTrigger value="timeline">Timeline View</TabsTrigger>
        </TabsList>

        <TabsContent value="organization" className="mt-6">
          {filteredSummaries.length > 0 ? (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
              {filteredSummaries.map((summary) => (
                <OrganizationJobsSection
                  key={summary.organization.id}
                  summary={summary}
                />
              ))}
            </div>
          ) : (
            <div className="text-center py-12">
              <AlertCircle className="h-12 w-12 mx-auto mb-3 text-muted-foreground opacity-50" />
              <p className="text-muted-foreground">
                No jobs found matching the selected filter
              </p>
            </div>
          )}
        </TabsContent>

        <TabsContent value="timeline" className="mt-6">
          <TimelineView jobs={filteredTimelineJobs} />
        </TabsContent>
      </Tabs>
    </div>
  );
}
