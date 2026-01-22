'use client';

import { useState } from 'react';
import { Copy, Check } from 'lucide-react';

interface CodeBlockProps {
  children: string;
  className?: string;
  showLineNumbers?: boolean;
  customHeader?: string;
  hideCopyAll?: boolean;
}

export function CodeBlock({ children, className, showLineNumbers = false, customHeader, hideCopyAll = false }: CodeBlockProps) {
  const [copiedLine, setCopiedLine] = useState<number | null>(null);
  const [hoveredLine, setHoveredLine] = useState<number | null>(null);

  const language = className?.replace(/^language-/, '') || '';
  const content = typeof children === 'string' ? children : String(children);
  const lines = content.split('\n').filter((line, index, arr) => {
    // Remove last empty line if it exists
    return index !== arr.length - 1 || line.trim() !== '';
  });

  const handleCopyLine = async (line: string, index: number) => {
    try {
      await navigator.clipboard.writeText(line);
      setCopiedLine(index);
      setTimeout(() => setCopiedLine(null), 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  };

  const getLineClassName = (line: string) => {
    const trimmedLine = line.trim();
    if (trimmedLine.startsWith('# ✓')) {
      return 'code-comment-success';
    } else if (trimmedLine.startsWith('#')) {
      return 'code-comment';
    }
    return '';
  };

  return (
    <div className="code-block-wrapper group relative mb-6 mt-6">
      <div className="overflow-hidden rounded-lg border border-border bg-muted/80 dark:bg-muted/60 shadow-sm backdrop-blur-sm transition-all duration-200 hover:border-border hover:shadow-md">
        {/* Header */}
        <div className="flex items-center justify-between border-b-2 border-border bg-secondary dark:bg-secondary/90 px-4 py-2.5 backdrop-blur-sm">
          {/* Language badge or custom header */}
          <div className="flex items-center gap-2">
            {customHeader ? (
              <span className="text-xs font-bold uppercase tracking-wider text-foreground">
                {customHeader}
              </span>
            ) : language ? (
              <span className="text-xs font-bold uppercase tracking-wider text-foreground">
                {language}
              </span>
            ) : (
              <span className="text-xs font-bold uppercase tracking-wider text-foreground">
                Code
              </span>
            )}
          </div>

          {/* Copy all button */}
          {!hideCopyAll && (
            <button
              className="
                flex items-center gap-1.5 rounded-md px-2.5 py-1 text-xs font-medium
                text-muted-foreground transition-all duration-200
                hover:bg-background/80 hover:text-foreground hover:shadow-sm
              "
              onClick={() => {
                navigator.clipboard.writeText(content);
                setCopiedLine(-1);
                setTimeout(() => setCopiedLine(null), 2000);
              }}
              aria-label="Copy all code"
            >
              {copiedLine === -1 ? (
                <>
                  <Check className="h-3.5 w-3.5" />
                  <span>Copied</span>
                </>
              ) : (
                <>
                  <Copy className="h-3.5 w-3.5" />
                  <span>Copy all</span>
                </>
              )}
            </button>
          )}
        </div>

        {/* Code content */}
        <div className="overflow-x-auto">
          <code className="block font-mono text-[13px] leading-relaxed">
            {lines.map((line, index) => {
              const isHovered = hoveredLine === index;
              const isCopied = copiedLine === index;
              const lineNumber = index + 1;
              const isEmpty = line.trim() === '';
              const isEven = index % 2 === 0;

              return (
                <div
                  key={index}
                  className={`
                    group/line relative flex min-h-[32px] w-full items-center transition-colors duration-100
                    ${!isEmpty ? 'cursor-pointer' : ''}
                    ${isEven ? 'bg-slate-200/60 dark:bg-muted/45' : 'bg-white dark:bg-muted/30'}
                    ${isHovered && !isEmpty ? '!bg-primary/25 dark:!bg-primary/20' : ''}
                    ${isCopied ? '!bg-primary/30 dark:!bg-primary/25' : ''}
                  `}
                  onMouseEnter={() => !isEmpty && setHoveredLine(index)}
                  onMouseLeave={() => setHoveredLine(null)}
                  onClick={() => !isEmpty && handleCopyLine(line, index)}
                >
                    {/* Line number */}
                    {showLineNumbers && (
                      <span className="sticky left-0 inline-block w-12 flex-shrink-0 select-none px-4 py-2 text-right text-xs text-muted-foreground/60 backdrop-blur-sm">
                        {lineNumber}
                      </span>
                    )}

                    {/* Code content */}
                    <span className={`flex-1 px-4 py-2 ${getLineClassName(line)}`}>
                      {line || ' '}
                    </span>

                    {/* Copy button - appears on hover */}
                    {!isEmpty && (
                      <button
                        className={`
                          absolute right-3 flex items-center gap-1.5 rounded-md border border-border/60
                          bg-background/95 px-2.5 py-1 text-xs font-medium
                          text-muted-foreground shadow-sm backdrop-blur-sm
                          transition-all duration-150
                          hover:border-border hover:bg-background hover:text-foreground hover:shadow
                          ${isHovered || isCopied ? 'opacity-100 translate-x-0' : 'opacity-0 translate-x-2 pointer-events-none'}
                        `}
                        onClick={(e) => {
                          e.stopPropagation();
                          handleCopyLine(line, index);
                        }}
                        aria-label="Copy line"
                      >
                        {isCopied ? (
                          <>
                            <Check className="h-3 w-3 text-green-600 dark:text-green-400" />
                            <span className="text-green-600 dark:text-green-400">Copied!</span>
                          </>
                        ) : (
                          <>
                            <Copy className="h-3 w-3" />
                            <span>Copy</span>
                          </>
                        )}
                      </button>
                    )}
                  </div>
                );
              })}
            </code>
          </div>
      </div>
    </div>
  );
}
