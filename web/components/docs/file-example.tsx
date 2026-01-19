'use client';

interface FileExampleProps {
  filename: string;
  children: string;
  language?: string;
}

export function FileExample({ filename, children, language = 'text' }: FileExampleProps) {
  let processedContent = children;

  // Remove single leading newline if present
  if (processedContent.startsWith('\n')) {
    processedContent = processedContent.substring(1);
  }

  // Remove single trailing newline if present
  if (processedContent.endsWith('\n')) {
    processedContent = processedContent.substring(0, processedContent.length - 1);
  }

  // Remove common leading indentation from all lines
  const lines = processedContent.split('\n');

  // Find minimum indentation (ignoring empty lines)
  const minIndent = lines
    .filter(line => line.trim().length > 0)
    .reduce((min, line) => {
      const match = line.match(/^(\s*)/);
      const indent = match ? match[1].length : 0;
      return Math.min(min, indent);
    }, Infinity);

  // Remove the common indentation from all lines
  if (minIndent > 0 && minIndent !== Infinity) {
    processedContent = lines
      .map(line => line.substring(minIndent))
      .join('\n');
  }

  // Special handling for BibTeX: ensure proper indentation
  if (language === 'bibtex' && processedContent.trim().startsWith('@')) {
    const bibtexLines = processedContent.split('\n');
    const formattedLines = bibtexLines.map((line, index) => {
      const trimmed = line.trim();
      // First line (@misc, @article, etc.) - no indent
      if (index === 0) return trimmed;
      // Closing brace - no indent
      if (trimmed === '}') return trimmed;
      // Empty lines - keep empty
      if (trimmed === '') return '';
      // Field lines - indent with 2 spaces
      return '  ' + trimmed;
    });
    processedContent = formattedLines.join('\n');
  }

  return (
    <div className="not-prose my-4 rounded-lg border border-border overflow-hidden">
      {/* File header */}
      <div className="bg-muted/50 border-b border-border px-4 py-2 flex items-center gap-2">
        <svg
          className="w-4 h-4 text-muted-foreground"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
          />
        </svg>
        <span className="text-sm font-mono text-foreground">{filename}</span>
      </div>

      {/* File content */}
      <pre
        className="m-0 overflow-x-auto bg-muted p-4 rounded-none border-0 font-mono text-sm"
        style={{
          tabSize: 2,
          whiteSpace: 'pre',
        }}
      >
        {processedContent}
      </pre>
    </div>
  );
}
