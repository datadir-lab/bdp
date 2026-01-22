'use client';

import * as React from 'react';
import * as TooltipPrimitive from '@radix-ui/react-tooltip';
import * as DialogPrimitive from '@radix-ui/react-dialog';

export default function TestRadixRawPage() {
  const [tooltipOpen, setTooltipOpen] = React.useState(false);
  const [dialogOpen, setDialogOpen] = React.useState(false);

  React.useEffect(() => {
    console.log('[RAW TEST] Tooltip open:', tooltipOpen);
  }, [tooltipOpen]);

  React.useEffect(() => {
    console.log('[RAW TEST] Dialog open:', dialogOpen);
  }, [dialogOpen]);

  return (
    <div className="container py-20 space-y-12">
      <h1 className="text-3xl font-bold mb-8">Raw Radix UI Test (No Shadcn Wrappers)</h1>

      {/* Test 1: Raw Tooltip - Uncontrolled */}
      <div className="space-y-4">
        <h2 className="text-xl font-semibold">Test 1: Raw Uncontrolled Tooltip</h2>
        <TooltipPrimitive.Provider>
          <TooltipPrimitive.Root>
            <TooltipPrimitive.Trigger asChild>
              <button className="px-4 py-2 bg-blue-500 text-white rounded">
                Hover me (raw uncontrolled)
              </button>
            </TooltipPrimitive.Trigger>
            <TooltipPrimitive.Portal>
              <TooltipPrimitive.Content
                className="bg-black text-white px-3 py-2 rounded text-sm"
                sideOffset={5}
              >
                This is a raw Radix tooltip
                <TooltipPrimitive.Arrow className="fill-black" />
              </TooltipPrimitive.Content>
            </TooltipPrimitive.Portal>
          </TooltipPrimitive.Root>
        </TooltipPrimitive.Provider>
      </div>

      {/* Test 2: Raw Tooltip - Controlled */}
      <div className="space-y-4">
        <h2 className="text-xl font-semibold">Test 2: Raw Controlled Tooltip</h2>
        <TooltipPrimitive.Provider>
          <TooltipPrimitive.Root open={tooltipOpen} onOpenChange={setTooltipOpen}>
            <TooltipPrimitive.Trigger asChild>
              <button className="px-4 py-2 bg-green-500 text-white rounded">
                Hover me (raw controlled)
              </button>
            </TooltipPrimitive.Trigger>
            <TooltipPrimitive.Portal>
              <TooltipPrimitive.Content
                className="bg-black text-white px-3 py-2 rounded text-sm"
                sideOffset={5}
              >
                Controlled - check console
                <TooltipPrimitive.Arrow className="fill-black" />
              </TooltipPrimitive.Content>
            </TooltipPrimitive.Portal>
          </TooltipPrimitive.Root>
        </TooltipPrimitive.Provider>
        <p className="text-sm">Tooltip open: {tooltipOpen ? 'true' : 'false'}</p>
        <button
          onClick={() => setTooltipOpen(!tooltipOpen)}
          className="px-4 py-2 bg-gray-500 text-white rounded"
        >
          Toggle Tooltip
        </button>
      </div>

      {/* Test 3: Raw Dialog */}
      <div className="space-y-4">
        <h2 className="text-xl font-semibold">Test 3: Raw Dialog</h2>
        <DialogPrimitive.Root open={dialogOpen} onOpenChange={setDialogOpen}>
          <DialogPrimitive.Trigger asChild>
            <button className="px-4 py-2 bg-purple-500 text-white rounded">
              Open Raw Dialog
            </button>
          </DialogPrimitive.Trigger>
          <DialogPrimitive.Portal>
            <DialogPrimitive.Overlay className="fixed inset-0 bg-black/50" />
            <DialogPrimitive.Content className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-white p-6 rounded shadow-lg w-96">
              <DialogPrimitive.Title className="text-lg font-bold mb-4">
                Raw Dialog Test
              </DialogPrimitive.Title>
              <DialogPrimitive.Description className="mb-4">
                This is a completely raw Radix UI dialog with no wrappers.
              </DialogPrimitive.Description>
              <p className="mb-4">Dialog open state: {dialogOpen ? 'true' : 'false'}</p>
              <div className="flex gap-2">
                <DialogPrimitive.Close asChild>
                  <button className="px-4 py-2 bg-gray-500 text-white rounded">
                    Close (built-in)
                  </button>
                </DialogPrimitive.Close>
                <button
                  onClick={() => setDialogOpen(false)}
                  className="px-4 py-2 bg-red-500 text-white rounded"
                >
                  Close (manual)
                </button>
              </div>
            </DialogPrimitive.Content>
          </DialogPrimitive.Portal>
        </DialogPrimitive.Root>
      </div>
    </div>
  );
}
