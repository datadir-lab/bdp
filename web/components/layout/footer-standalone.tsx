'use client';

import Link from 'next/link';
import { Github } from 'lucide-react';
import { siteConfig, getGithubIssuesUrl, getGithubDiscussionsUrl, getGithubContributingUrl, getGithubLicenseUrl } from '@/lib/site-config';

// Standalone footer for root 404 page (no i18n context)
export function FooterStandalone() {
  const currentYear = new Date().getFullYear();

  const footerLinks = {
    resources: [
      { name: 'GitHub', href: siteConfig.github.url },
      { name: 'Issues', href: getGithubIssuesUrl() },
      { name: 'Discussions', href: getGithubDiscussionsUrl() },
      { name: 'Contributing', href: getGithubContributingUrl() },
    ],
    legal: [
      { name: 'Privacy Policy', href: '/privacy' },
      { name: 'Terms of Service', href: '/terms' },
      { name: 'License', href: getGithubLicenseUrl() },
    ],
  };

  return (
    <footer className="border-t bg-background">
      <div className="container mx-auto py-12 md:py-16">
        <div className="grid grid-cols-2 gap-8 md:grid-cols-3 md:gap-12 lg:gap-16">
          {/* Brand */}
          <div className="col-span-2 md:col-span-1">
            <span className="font-mono font-bold text-3xl tracking-tight select-none text-foreground">
              [bdp]
            </span>
            <p className="mt-4 text-sm text-muted-foreground">
              A comprehensive platform for managing bioinformatics software dependencies.
            </p>
            <div className="mt-4 flex gap-4">
              <Link
                href={siteConfig.github.url}
                target="_blank"
                rel="noopener noreferrer"
                className="text-muted-foreground transition-colors hover:text-foreground"
              >
                <Github className="h-5 w-5" />
                <span className="sr-only">GitHub</span>
              </Link>
            </div>
          </div>

          {/* Resources Links */}
          <div>
            <h3 className="mb-4 text-sm font-semibold">Resources</h3>
            <ul className="space-y-3">
              {footerLinks.resources.map((link) => (
                <li key={link.name}>
                  <Link
                    href={link.href}
                    target={link.href.startsWith('http') ? '_blank' : undefined}
                    rel={
                      link.href.startsWith('http')
                        ? 'noopener noreferrer'
                        : undefined
                    }
                    className="text-sm text-muted-foreground transition-colors hover:text-foreground"
                  >
                    {link.name}
                  </Link>
                </li>
              ))}
            </ul>
          </div>

          {/* Legal Links */}
          <div>
            <h3 className="mb-4 text-sm font-semibold">Legal</h3>
            <ul className="space-y-3">
              {footerLinks.legal.map((link) => (
                <li key={link.name}>
                  <Link
                    href={link.href}
                    target={link.href.startsWith('http') ? '_blank' : undefined}
                    rel={
                      link.href.startsWith('http')
                        ? 'noopener noreferrer'
                        : undefined
                    }
                    className="text-sm text-muted-foreground transition-colors hover:text-foreground"
                  >
                    {link.name}
                  </Link>
                </li>
              ))}
            </ul>
          </div>
        </div>

        {/* Bottom Bar */}
        <div className="mt-12 pt-8">
          <p className="text-center text-sm text-muted-foreground">
            © {currentYear} {siteConfig.name}. All rights reserved.
          </p>
        </div>
      </div>
    </footer>
  );
}
