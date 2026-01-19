'use client';

import * as React from 'react';
import { Menu } from 'lucide-react';
import { Button } from '@/components/ui/button';
import * as Dialog from '@radix-ui/react-dialog';
import { X } from 'lucide-react';
import { DocsSidebar } from './DocsSidebar';
import { DocsSearch } from '@/components/shared/docs-search';
import { usePathname } from '@/i18n/navigation';

export function MobileDocsNav() {
  const [open, setOpen] = React.useState(false);
  const pathname = usePathname();

  React.useEffect(() => {
    setOpen(false);
  }, [pathname]);

  return (
    <Dialog.Root open={open} onOpenChange={setOpen} modal>
      <Dialog.Trigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className="md:hidden"
          aria-label="Open navigation menu"
        >
          <Menu className="h-5 w-5" />
        </Button>
      </Dialog.Trigger>

      {open && (
        <Dialog.Portal>
          <Dialog.Overlay className="fixed inset-0 z-50 bg-black/80 animate-in fade-in-0 duration-300" />
          <Dialog.Content className="fixed left-0 top-0 z-[51] h-full w-[280px] sm:w-[320px] border-r bg-background p-6 shadow-lg animate-in slide-in-from-left duration-300 overflow-y-auto">
          <Dialog.Close className="absolute right-4 top-4 rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none">
            <X className="h-4 w-4" />
            <span className="sr-only">Close</span>
          </Dialog.Close>

          <Dialog.Title className="text-lg font-semibold text-foreground pb-4">
            Documentation
          </Dialog.Title>

          <div className="mb-6">
            <DocsSearch />
          </div>

          <DocsSidebar onLinkClick={() => setOpen(false)} />
        </Dialog.Content>
      </Dialog.Portal>
      )}
    </Dialog.Root>
  );
}
