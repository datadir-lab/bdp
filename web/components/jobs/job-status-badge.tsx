import * as React from 'react';
import { Clock, Loader2, CheckCircle, XCircle } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import type { JobStatus } from '@/lib/types/job';

export interface JobStatusBadgeProps {
  status: JobStatus;
  size?: 'sm' | 'md' | 'lg';
  showIcon?: boolean;
  className?: string;
}

const statusConfig: Record<string, {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  variant: 'outline' | 'default' | 'destructive';
  className: string;
}> = {
  // New ingestion_jobs statuses
  pending: {
    icon: Clock,
    label: 'Pending',
    variant: 'outline' as const,
    className: 'border-muted-foreground text-muted-foreground',
  },
  downloading: {
    icon: Loader2,
    label: 'Downloading',
    variant: 'default' as const,
    className: 'bg-blue-500 text-white',
  },
  download_verified: {
    icon: CheckCircle,
    label: 'Download Verified',
    variant: 'default' as const,
    className: 'bg-blue-400 text-white',
  },
  parsing: {
    icon: Loader2,
    label: 'Parsing',
    variant: 'default' as const,
    className: 'bg-purple-500 text-white',
  },
  storing: {
    icon: Loader2,
    label: 'Storing',
    variant: 'default' as const,
    className: 'bg-indigo-500 text-white',
  },
  completed: {
    icon: CheckCircle,
    label: 'Completed',
    variant: 'default' as const,
    className: 'bg-green-500 text-white hover:bg-green-500/80',
  },
  failed: {
    icon: XCircle,
    label: 'Failed',
    variant: 'destructive' as const,
    className: '',
  },
  // Legacy apalis statuses for backwards compatibility
  Pending: {
    icon: Clock,
    label: 'Pending',
    variant: 'outline' as const,
    className: 'border-muted-foreground text-muted-foreground',
  },
  Running: {
    icon: Loader2,
    label: 'Running',
    variant: 'default' as const,
    className: 'bg-blue-500 text-white animate-pulse',
  },
  Done: {
    icon: CheckCircle,
    label: 'Completed',
    variant: 'default' as const,
    className: 'bg-green-500 text-white hover:bg-green-500/80',
  },
  Failed: {
    icon: XCircle,
    label: 'Failed',
    variant: 'destructive' as const,
    className: '',
  },
};

const sizeConfig = {
  sm: {
    badge: 'text-xs px-2 py-0.5',
    icon: 'h-3 w-3',
  },
  md: {
    badge: 'text-sm px-2.5 py-0.5',
    icon: 'h-4 w-4',
  },
  lg: {
    badge: 'text-base px-3 py-1',
    icon: 'h-5 w-5',
  },
};

export function JobStatusBadge({
  status,
  size = 'md',
  showIcon = true,
  className,
}: JobStatusBadgeProps) {
  const config = statusConfig[status] || statusConfig.pending;
  const sizeStyles = sizeConfig[size];
  const Icon = config.icon;

  const isAnimated = ['downloading', 'parsing', 'storing', 'Running'].includes(status);

  return (
    <Badge
      variant={config.variant}
      className={cn(sizeStyles.badge, config.className, className)}
    >
      {showIcon && (
        <Icon
          className={cn(
            sizeStyles.icon,
            'mr-1',
            isAnimated && 'animate-spin'
          )}
          aria-hidden="true"
        />
      )}
      {config.label}
    </Badge>
  );
}
