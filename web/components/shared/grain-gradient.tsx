import { cn } from '@/lib/utils';

interface GrainGradientProps {
  className?: string;
  variant?: 'hero' | 'radial' | 'custom';
  children?: React.ReactNode;
}

export function GrainGradient({ className, variant = 'hero', children }: GrainGradientProps) {
  return (
    <div
      className={cn(
        'relative overflow-hidden',
        variant === 'hero' && 'gradient-radial-hero',
        variant === 'radial' && 'gradient-radial',
        className
      )}
    >
      <div className="grain-visible absolute inset-0" />
      {children && <div className="relative z-10">{children}</div>}
    </div>
  );
}
