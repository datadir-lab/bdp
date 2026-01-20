'use client';

import { Link } from '@/i18n/navigation';
import type { ComponentProps } from 'react';

export function SafeLink({ href, children, ...props }: ComponentProps<typeof Link>) {
  // Log warning if href contains undefined
  if (typeof href === 'string' && href.includes('undefined')) {
    console.error('SafeLink: Invalid href detected:', href);
    console.trace('Stack trace:');
  }

  return (
    <Link href={href} {...props}>
      {children}
    </Link>
  );
}
