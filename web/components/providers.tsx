'use client';

import * as React from 'react';
import { ThemeProvider as NextThemesProvider } from 'next-themes';
import type { ThemeProviderProps } from 'next-themes';
import { TooltipProvider as RadixTooltipProvider } from '@/components/ui/tooltip';

export function ThemeProvider({ children, ...props }: ThemeProviderProps) {
  return (
    <NextThemesProvider {...props}>
      <RadixTooltipProvider>
        {children}
      </RadixTooltipProvider>
    </NextThemesProvider>
  );
}
