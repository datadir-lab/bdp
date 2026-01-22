import * as React from 'react';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';

interface SourceTypeBadgeProps {
  sourceType: string;
  className?: string;
}

/**
 * Unified component for displaying source type badges with consistent colors across the app
 */
export function SourceTypeBadge({ sourceType, className }: SourceTypeBadgeProps) {
  const getSourceTypeBadgeClass = (type: string) => {
    const baseClasses = 'text-xs capitalize';
    switch (type.toLowerCase()) {
      case 'protein':
        return `${baseClasses} bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200 hover:bg-purple-100`;
      case 'genome':
        return `${baseClasses} bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200 hover:bg-green-100`;
      case 'organism':
      case 'taxonomy':
        return `${baseClasses} bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200 hover:bg-blue-100`;
      case 'transcript':
        return `${baseClasses} bg-teal-100 text-teal-800 dark:bg-teal-900 dark:text-teal-200 hover:bg-teal-100`;
      case 'annotation':
        return `${baseClasses} bg-amber-100 text-amber-800 dark:bg-amber-900 dark:text-amber-200 hover:bg-amber-100`;
      case 'structure':
        return `${baseClasses} bg-pink-100 text-pink-800 dark:bg-pink-900 dark:text-pink-200 hover:bg-pink-100`;
      case 'pathway':
        return `${baseClasses} bg-indigo-100 text-indigo-800 dark:bg-indigo-900 dark:text-indigo-200 hover:bg-indigo-100`;
      case 'bundle':
        return `${baseClasses} bg-cyan-100 text-cyan-800 dark:bg-cyan-900 dark:text-cyan-200 hover:bg-cyan-100`;
      default:
        return `${baseClasses} bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-200 hover:bg-gray-100`;
    }
  };

  return (
    <Badge className={cn(getSourceTypeBadgeClass(sourceType), className)}>
      {sourceType}
    </Badge>
  );
}
