'use client';

import * as React from 'react';
import { Copy, Check } from 'lucide-react';
import { CodeBlock } from '@/components/docs/code-block';
import { formatSourceId } from '@/lib/utils';
import type { VersionFile } from '@/lib/types/data-source';

interface CliCommandsProps {
  org: string;
  name: string;
  version: string;
  files: VersionFile[];
}

export function CliCommands({ org, name, version, files }: CliCommandsProps) {
  const [copiedChecksum, setCopiedChecksum] = React.useState<string | null>(null);

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i];
  };

  const copyChecksum = async (checksum: string) => {
    await navigator.clipboard.writeText(checksum);
    setCopiedChecksum(checksum);
    setTimeout(() => setCopiedChecksum(null), 2000);
  };

  const commands = files
    .map((file) => `bdp source add ${formatSourceId(org, name, file.format, version)}`)
    .join('\n');

  return (
    <div className="space-y-10">
      <div>
        <p className="text-sm text-muted-foreground mb-6">
          First, <a href="/docs/quick-start" className="text-primary hover:underline">initialize your project</a> if you haven't yet, then choose a format and run the command to add this data source:
        </p>

        <CodeBlock className="language-bash" customHeader="Add to your project" hideCopyAll={true}>
          {commands}
        </CodeBlock>
      </div>

      <div className="rounded-lg border-2 border-primary/20 bg-primary/5 p-5">
        <p className="text-sm font-medium">
          Need to install BDP CLI?{' '}
          <a
            href="/docs/installation"
            className="text-primary font-semibold hover:underline inline-flex items-center gap-1"
          >
            View installation guide →
          </a>
        </p>
      </div>

      {/* File Details */}
      <div className="space-y-4">
        <h3 className="text-sm font-medium text-muted-foreground">File Details</h3>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b text-xs text-muted-foreground">
                <th className="text-left py-2 pr-4 font-medium">Format</th>
                <th className="text-left py-2 pr-4 font-medium">Size</th>
                <th className="text-left py-2 pr-4 font-medium">Compression</th>
                <th className="text-left py-2 font-medium">Checksum</th>
              </tr>
            </thead>
            <tbody>
              {files.map((file) => (
                <tr key={file.id} className="border-b last:border-0">
                  <td className="py-2.5 pr-4">
                    <code className="text-xs font-medium">{file.format.toUpperCase()}</code>
                  </td>
                  <td className="py-2.5 pr-4 text-muted-foreground">
                    {formatBytes(file.size_bytes)}
                  </td>
                  <td className="py-2.5 pr-4 text-muted-foreground">
                    {file.compression && file.compression !== 'none' ? file.compression : '—'}
                  </td>
                  <td className="py-2.5">
                    <div className="flex items-center gap-2">
                      <code className="text-xs text-muted-foreground font-mono">
                        {file.checksum}
                      </code>
                      <button
                        onClick={() => copyChecksum(file.checksum)}
                        className="text-muted-foreground hover:text-foreground transition-colors"
                        aria-label="Copy checksum"
                      >
                        {copiedChecksum === file.checksum ? (
                          <Check className="h-3.5 w-3.5 text-green-600" />
                        ) : (
                          <Copy className="h-3.5 w-3.5" />
                        )}
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
