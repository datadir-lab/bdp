import type { MDXComponents } from 'mdx/types';
import Link from 'next/link';
import { CodeBlock } from '@/components/docs/code-block';

export function useMDXComponents(components: MDXComponents): MDXComponents {
  return {
    // Headings
    h1: ({ children, ...props }) => (
      <h1
        className="scroll-m-20 text-4xl font-bold tracking-tight mb-4 mt-8 first:mt-0"
        {...props}
      >
        {children}
      </h1>
    ),
    h2: ({ children, ...props }) => (
      <h2
        className="scroll-m-20 text-3xl font-semibold tracking-tight mb-3 mt-8 pb-2 border-b"
        {...props}
      >
        {children}
      </h2>
    ),
    h3: ({ children, ...props }) => (
      <h3
        className="scroll-m-20 text-2xl font-semibold tracking-tight mb-3 mt-6"
        {...props}
      >
        {children}
      </h3>
    ),
    h4: ({ children, ...props }) => (
      <h4
        className="scroll-m-20 text-xl font-semibold tracking-tight mb-2 mt-4"
        {...props}
      >
        {children}
      </h4>
    ),
    h5: ({ children, ...props }) => (
      <h5
        className="scroll-m-20 text-lg font-semibold tracking-tight mb-2 mt-4"
        {...props}
      >
        {children}
      </h5>
    ),
    h6: ({ children, ...props }) => (
      <h6
        className="scroll-m-20 text-base font-semibold tracking-tight mb-2 mt-4"
        {...props}
      >
        {children}
      </h6>
    ),

    // Text
    p: ({ children, ...props }) => (
      <p className="leading-7 mb-4 text-foreground/90" {...props}>
        {children}
      </p>
    ),
    strong: ({ children, ...props }) => (
      <strong className="font-semibold text-foreground" {...props}>
        {children}
      </strong>
    ),
    em: ({ children, ...props }) => (
      <em className="italic text-foreground/90" {...props}>
        {children}
      </em>
    ),

    // Links
    a: ({ href, children, ...props }) => {
      const isExternal = href?.startsWith('http');
      const isAnchor = href?.startsWith('#');

      if (isExternal) {
        return (
          <a
            href={href}
            target="_blank"
            rel="noopener noreferrer"
            className="text-primary hover:underline font-medium"
            {...props}
          >
            {children}
          </a>
        );
      }

      if (isAnchor) {
        return (
          <a
            href={href}
            className="text-primary hover:underline font-medium"
            {...props}
          >
            {children}
          </a>
        );
      }

      return (
        <Link
          href={href || '#'}
          className="text-primary hover:underline font-medium"
          {...props}
        >
          {children}
        </Link>
      );
    },

    // Lists
    ul: ({ children, ...props }) => (
      <ul className="my-4 ml-6 list-disc space-y-2" {...props}>
        {children}
      </ul>
    ),
    ol: ({ children, ...props }) => (
      <ol className="my-4 ml-6 list-decimal space-y-2" {...props}>
        {children}
      </ol>
    ),
    li: ({ children, ...props }) => (
      <li className="leading-7 text-foreground/90" {...props}>
        {children}
      </li>
    ),

    // Code
    code: ({ children, ...props }) => (
      <code
        className="relative rounded bg-muted px-[0.3rem] py-[0.2rem] font-mono text-sm"
        {...props}
      >
        {children}
      </code>
    ),
    pre: ({ children, ...props }) => {
      // Check if children is a code element
      if (
        children &&
        typeof children === 'object' &&
        'props' in children
      ) {
        const className = (children as any).props?.className || '';
        const code = (children as any).props?.children || '';

        return <CodeBlock className={className}>{code}</CodeBlock>;
      }

      // Fallback for non-code pre blocks
      return (
        <pre
          className="mb-4 mt-4 overflow-x-auto rounded-lg border bg-muted p-4"
          {...props}
        >
          {children}
        </pre>
      );
    },

    // Tables - enhanced for better display
    table: ({ children, ...props }) => (
      <div className="my-6 w-full overflow-x-auto">
        <table className="w-full" {...props}>
          {children}
        </table>
      </div>
    ),
    thead: ({ children, ...props }) => (
      <thead className="bg-muted/50" {...props}>
        {children}
      </thead>
    ),
    tbody: ({ children, ...props }) => (
      <tbody {...props}>
        {children}
      </tbody>
    ),
    tr: ({ children, ...props }) => (
      <tr className="m-0 border-t p-0 even:bg-muted/30" {...props}>
        {children}
      </tr>
    ),
    th: ({ children, ...props }) => (
      <th
        className="border border-border px-4 py-2 text-left font-bold [&[align=center]]:text-center [&[align=right]]:text-right"
        {...props}
      >
        {children}
      </th>
    ),
    td: ({ children, ...props }) => (
      <td
        className="border border-border px-4 py-2 text-left [&[align=center]]:text-center [&[align=right]]:text-right"
        {...props}
      >
        {children}
      </td>
    ),

    // Block elements
    blockquote: ({ children, ...props }) => (
      <blockquote
        className="mt-4 mb-4 border-l-4 border-primary pl-4 italic text-foreground/80"
        {...props}
      >
        {children}
      </blockquote>
    ),
    hr: ({ ...props }) => (
      <hr className="my-8 border-border" {...props} />
    ),

    // Images
    img: ({ src, alt, ...props }) => (
      // eslint-disable-next-line @next/next/no-img-element
      <img
        src={src}
        alt={alt || ''}
        className="rounded-lg border my-4"
        {...props}
      />
    ),

    ...components,
  };
}
