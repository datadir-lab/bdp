'use client';

import * as React from 'react';

/**
 * Global fix for Radix UI pointer-events bug
 *
 * Issue: Radix UI applies `pointer-events: none` to body when dialogs/tooltips open,
 * but sometimes fails to remove it when they close, leaving the entire page unresponsive.
 *
 * This is a known bug in Radix UI, particularly with React 19:
 * - https://github.com/radix-ui/primitives/issues/3445
 * - https://github.com/radix-ui/primitives/issues/3648
 * - https://github.com/radix-ui/primitives/issues/1241
 *
 * This component watches for the bug and forcibly cleans it up.
 */
export function PortalCleanupFix() {
  React.useEffect(() => {
    let animationFrameId: number;

    const checkAndFixPointerEvents = () => {
      const body = document.body;
      const style = body.getAttribute('style') || '';

      // Check if body has pointer-events: none
      if (style.includes('pointer-events: none') || style.includes('pointer-events:none')) {
        // Check if there are any open radix portals/overlays
        const hasOpenDialog = document.querySelector('[data-state="open"][data-radix-dialog-content]');
        const hasOpenPopover = document.querySelector('[data-state="open"][data-radix-popover-content]');
        const hasOpenTooltip = document.querySelector('[data-state="open"][data-radix-tooltip-content]');

        const hasAnyOpenPortal = hasOpenDialog || hasOpenPopover || hasOpenTooltip;

        // If no portals are open, but pointer-events is still none, forcibly remove it
        if (!hasAnyOpenPortal) {
          console.warn('[PORTAL-CLEANUP-FIX] Detected orphaned pointer-events: none, cleaning up...');

          // Remove pointer-events from inline styles
          const newStyle = style
            .replace(/pointer-events\s*:\s*none\s*;?/g, '')
            .replace(/;\s*;/g, ';')
            .trim();

          if (newStyle) {
            body.setAttribute('style', newStyle);
          } else {
            body.removeAttribute('style');
          }
        }
      }

      // Check continuously (RAF is more performant than setInterval)
      animationFrameId = requestAnimationFrame(checkAndFixPointerEvents);
    };

    // Start checking
    animationFrameId = requestAnimationFrame(checkAndFixPointerEvents);

    return () => {
      cancelAnimationFrame(animationFrameId);
    };
  }, []);

  return null;
}
