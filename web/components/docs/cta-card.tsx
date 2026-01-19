'use client';

import Link from 'next/link';

interface CtaCardProps {
  heading: string;
  text: string;
  linkText: string;
  linkHref: string;
}

export function CtaCard({ heading, text, linkText, linkHref }: CtaCardProps) {
  return (
    <div className="not-prose my-8 rounded-lg border-2 border-primary/20 bg-primary/5 p-8 text-center">
      <h2 className="text-2xl font-bold mb-3 text-foreground">{heading}</h2>
      <p className="text-muted-foreground mb-6 text-lg">{text}</p>
      <Link
        href={linkHref}
        className="inline-flex items-center justify-center rounded-md bg-primary px-8 py-3 text-base font-semibold text-primary-foreground shadow-sm hover:bg-primary/90 transition-colors no-underline"
      >
        {linkText}
      </Link>
    </div>
  );
}
