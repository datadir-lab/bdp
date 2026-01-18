'use client';

import { ChevronLeft, ChevronRight } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { SearchPagination as SearchPaginationType } from '@/lib/types/search';

interface SearchPaginationProps {
  pagination: SearchPaginationType;
  onPageChange: (page: number) => void;
}

export function SearchPagination({ pagination, onPageChange }: SearchPaginationProps) {
  const { page, pages, has_next, has_prev, total, per_page } = pagination;

  if (total === 0) {
    return null;
  }

  const startItem = (page - 1) * per_page + 1;
  const endItem = Math.min(page * per_page, total);

  return (
    <div className="flex items-center justify-between border-t pt-4">
      <div className="text-sm text-muted-foreground">
        Showing <span className="font-medium">{startItem}</span> to{' '}
        <span className="font-medium">{endItem}</span> of{' '}
        <span className="font-medium">{total}</span> results
      </div>

      <div className="flex items-center gap-2">
        <div className="text-sm text-muted-foreground">
          Page <span className="font-medium">{page}</span> of{' '}
          <span className="font-medium">{pages}</span>
        </div>

        <div className="flex items-center gap-1">
          <Button
            variant="outline"
            size="sm"
            onClick={() => onPageChange(page - 1)}
            disabled={!has_prev}
            className="h-8 w-8 p-0"
          >
            <ChevronLeft className="h-4 w-4" />
            <span className="sr-only">Previous page</span>
          </Button>

          <Button
            variant="outline"
            size="sm"
            onClick={() => onPageChange(page + 1)}
            disabled={!has_next}
            className="h-8 w-8 p-0"
          >
            <ChevronRight className="h-4 w-4" />
            <span className="sr-only">Next page</span>
          </Button>
        </div>
      </div>
    </div>
  );
}
