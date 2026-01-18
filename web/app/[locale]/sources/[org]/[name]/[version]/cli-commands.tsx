'use client';

import * as React from 'react';
import { Button } from '@/components/ui/button';
import { Check, Copy } from 'lucide-react';
import type { VersionFile } from '@/lib/types/data-source';

interface CliCommandsProps {
  org: string;
  name: string;
  version: string;
  files: VersionFile[];
}

export function CliCommands({ org, name, version, files }: CliCommandsProps) {
  const [copiedIndex, setCopiedIndex] = React.useState<number | null>(null);

  const handleCopy = async (command: string, index: number) => {
    try {
      await navigator.clipboard.writeText(command);
      setCopiedIndex(index);
      setTimeout(() => setCopiedIndex(null), 2000);
    } catch (error) {
      console.error('Failed to copy:', error);
    }
  };

  return (
    <div className="space-y-3">
      <p className="text-sm text-muted-foreground">
        Add this data source to your project with the BDP CLI. Choose a format below:
      </p>

      <div className="space-y-2">
        {files.map((file, index) => {
          const command = `bdp source add ${org}:${name}@${version}-${file.format}`;
          const isCopied = copiedIndex === index;

          return (
            <div
              key={file.id}
              className="flex items-center gap-2 rounded-lg border bg-muted/50 p-3"
            >
              <code className="flex-1 text-sm font-mono">{command}</code>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => handleCopy(command, index)}
                className="shrink-0"
              >
                {isCopied ? (
                  <>
                    <Check className="h-4 w-4 mr-1" />
                    Copied
                  </>
                ) : (
                  <>
                    <Copy className="h-4 w-4 mr-1" />
                    Copy
                  </>
                )}
              </Button>
            </div>
          );
        })}
      </div>

      <div className="rounded-lg border bg-card p-4 text-sm">
        <p className="font-medium mb-2">Don't have BDP CLI installed?</p>
        <p className="text-muted-foreground mb-3">
          Install it with a single command:
        </p>
        <code className="block rounded bg-muted px-3 py-2 font-mono text-sm">
          curl --proto '=https' --tlsv1.2 -LsSf
          https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.sh | sh
        </code>
      </div>
    </div>
  );
}
