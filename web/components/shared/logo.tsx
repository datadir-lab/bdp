import { Link } from '@/i18n/navigation';
import { cn } from '@/lib/utils';

interface LogoProps {
  className?: string;
  size?: 'sm' | 'md' | 'lg';
  linkable?: boolean;
}

export function Logo({ className, size = 'md', linkable = true }: LogoProps) {
  const sizeClasses = {
    sm: 'text-lg',
    md: 'text-2xl',
    lg: 'text-3xl',
  };

  const logoElement = (
    <span
      className={cn(
        'font-mono font-bold tracking-tight select-none text-foreground',
        sizeClasses[size],
        className
      )}
    >
      [bdp]
    </span>
  );

  if (linkable) {
    return (
      <Link href="/" className="no-underline hover:no-underline transition-opacity hover:opacity-80">
        {logoElement}
      </Link>
    );
  }

  return logoElement;
}
