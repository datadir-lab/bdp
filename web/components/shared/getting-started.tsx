'use client';

import React, { useState } from 'react';
import { Copy, Check } from 'lucide-react';
import { cn } from '@/lib/utils';
import { siteConfig } from '@/lib/site-config';

interface TerminalLineProps {
  command: string;
  prompt?: string;
  comment?: string;
}

function TerminalLine({ command, prompt = '$', comment }: TerminalLineProps) {
  const [copied, setCopied] = React.useState(false);

  const handleCopy = (e: React.MouseEvent) => {
    e.stopPropagation();
    void navigator.clipboard.writeText(command).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };

  const handleLineClick = () => {
    void navigator.clipboard.writeText(command).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };

  return (
    <div
      className="group flex items-start gap-3 px-6 py-4 cursor-pointer"
      onClick={handleLineClick}
    >
      <div className="flex items-center gap-2 text-muted-foreground font-mono text-base select-none">
        <span>{prompt}</span>
      </div>
      <code className="flex-1 font-mono text-base text-foreground select-all bg-transparent text-left">
        {command}
        {comment && <span className="text-muted-foreground/50 ml-2 text-sm">{comment}</span>}
      </code>
      <button
        onClick={handleCopy}
        className={cn(
          'p-2 rounded text-muted-foreground hover:text-foreground transition-all',
          'opacity-0 group-hover:opacity-100'
        )}
        aria-label="Copy to clipboard"
      >
        {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
      </button>
    </div>
  );
}

interface TerminalWindowProps {
  children: React.ReactNode;
  platform: 'unix' | 'windows';
  activeTab: 'unix' | 'windows';
  onTabChange: (tab: 'unix' | 'windows') => void;
}

function TerminalWindow({ children, platform, activeTab, onTabChange }: TerminalWindowProps) {
  return (
    <div className="rounded-lg border border-border bg-card overflow-hidden font-mono shadow-lg">
      {/* Terminal header */}
      <div className="flex items-center justify-between px-6 py-3 border-b border-border bg-muted/30 min-h-[48px]">
        {/* Left side - Window controls */}
        <div className="flex items-center gap-2">
          {platform === 'windows' ? (
            <div className="flex items-center gap-2 h-3">
              <div className="text-xs text-muted-foreground font-sans leading-none">PowerShell</div>
            </div>
          ) : (
            <div className="flex gap-1.5 h-3">
              <div className="w-3 h-3 rounded-full bg-destructive/80" />
              <div className="w-3 h-3 rounded-full bg-yellow-500/80" />
              <div className="w-3 h-3 rounded-full bg-green-500/80" />
            </div>
          )}
        </div>

        {/* Right side - Platform tabs */}
        <div className="flex gap-1 bg-muted/50 rounded p-1">
          <button
            onClick={() => onTabChange('unix')}
            className={cn(
              'px-3 py-1 text-xs font-medium rounded transition-colors',
              activeTab === 'unix'
                ? 'bg-background text-foreground shadow-sm'
                : 'text-muted-foreground hover:text-foreground'
            )}
          >
            Unix / macOS
          </button>
          <button
            onClick={() => onTabChange('windows')}
            className={cn(
              'px-3 py-1 text-xs font-medium rounded transition-colors',
              activeTab === 'windows'
                ? 'bg-background text-foreground shadow-sm'
                : 'text-muted-foreground hover:text-foreground'
            )}
          >
            Windows
          </button>
        </div>
      </div>
      {/* Terminal content */}
      <div>{children}</div>
    </div>
  );
}

export function GettingStarted() {
  const [activeTab, setActiveTab] = useState<'unix' | 'windows'>('unix');

  return (
    <div className="w-full max-w-4xl animate-fade-in">
      <TerminalWindow platform={activeTab} activeTab={activeTab} onTabChange={setActiveTab}>
        {activeTab === 'unix' ? (
          <>
            <TerminalLine
              command={siteConfig.install.unix}
              comment="# Install BDP"
            />
            <TerminalLine command="bdp init" comment="# Initialize project" />
            <TerminalLine command="bdp source add uniprot:P01308-fasta@1.0" comment="# Add data source" />
            <TerminalLine command="bdp pull" comment="# Download and cache" />
          </>
        ) : (
          <>
            <TerminalLine
              command={siteConfig.install.windows}
              prompt="PS>"
              comment="# Install BDP"
            />
            <TerminalLine command="bdp init" prompt="PS>" comment="# Initialize project" />
            <TerminalLine command="bdp source add uniprot:P01308-fasta@1.0" prompt="PS>" comment="# Add data source" />
            <TerminalLine command="bdp pull" prompt="PS>" comment="# Download and cache" />
          </>
        )}
      </TerminalWindow>
    </div>
  );
}
