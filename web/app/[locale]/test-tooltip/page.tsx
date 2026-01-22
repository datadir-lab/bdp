'use client';

import * as React from 'react';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';

export default function TestTooltipPage() {
  const [dialogOpen, setDialogOpen] = React.useState(false);
  const [tooltipOpen, setTooltipOpen] = React.useState(false);

  const handleTooltipOpenChange = React.useCallback((open: boolean) => {
    console.log('[TOOLTIP] onOpenChange called with:', open, 'Stack trace:', new Error().stack);
    setTooltipOpen(open);
  }, []);

  const handleDialogOpenChange = React.useCallback((open: boolean) => {
    console.log('[DIALOG] onOpenChange called with:', open, 'Stack trace:', new Error().stack);
    setDialogOpen(open);
  }, []);

  React.useEffect(() => {
    console.log('[TOOLTIP] State changed to:', tooltipOpen);
  }, [tooltipOpen]);

  React.useEffect(() => {
    console.log('[DIALOG] State changed to:', dialogOpen);
  }, [dialogOpen]);

  return (
    <div className="container py-20 space-y-12">
      <div>
        <h1 className="text-3xl font-bold mb-8">Tooltip & Modal Test</h1>

        {/* Test 1: Uncontrolled Tooltip */}
        <div className="space-y-4">
          <h2 className="text-xl font-semibold">Test 1: Uncontrolled Tooltip</h2>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="outline">Hover me (uncontrolled)</Button>
            </TooltipTrigger>
            <TooltipContent>
              <p>This tooltip should close when you move away</p>
            </TooltipContent>
          </Tooltip>
        </div>

        {/* Test 2: Controlled Tooltip */}
        <div className="space-y-4">
          <h2 className="text-xl font-semibold">Test 2: Controlled Tooltip</h2>
          <Tooltip open={tooltipOpen} onOpenChange={handleTooltipOpenChange}>
            <TooltipTrigger asChild>
              <Button variant="outline">Hover me (controlled)</Button>
            </TooltipTrigger>
            <TooltipContent>
              <p>This is controlled. Check console for state changes.</p>
            </TooltipContent>
          </Tooltip>
          <p className="text-sm text-muted-foreground">
            Tooltip open: {tooltipOpen ? 'true' : 'false'}
          </p>
          <Button
            onClick={() => {
              console.log('[TOGGLE BUTTON] Clicked, current state:', tooltipOpen);
              setTooltipOpen(!tooltipOpen);
            }}
          >
            Toggle Tooltip
          </Button>
        </div>

        {/* Test 3: Dialog */}
        <div className="space-y-4">
          <h2 className="text-xl font-semibold">Test 3: Dialog</h2>
          <Dialog open={dialogOpen} onOpenChange={handleDialogOpenChange}>
            <DialogTrigger asChild>
              <Button>Open Dialog</Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Test Dialog</DialogTitle>
                <DialogDescription>
                  This dialog should close when clicking outside or pressing escape.
                </DialogDescription>
              </DialogHeader>
              <p>Dialog open state: {dialogOpen ? 'true' : 'false'}</p>
              <Button
                onClick={() => {
                  console.log('[CLOSE BUTTON] Clicked, current state:', dialogOpen);
                  setDialogOpen(false);
                }}
              >
                Close
              </Button>
            </DialogContent>
          </Dialog>
        </div>
      </div>
    </div>
  );
}
