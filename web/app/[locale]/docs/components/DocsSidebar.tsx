'use client';

import { Link, usePathname } from '@/i18n/navigation';

interface NavItem {
  title: string;
  href: string;
  items?: NavItem[];
}

const docsNavigation: NavItem[] = [
  {
    title: 'Getting Started',
    href: '/docs',
    items: [
      { title: 'Introduction', href: '/docs' },
      { title: 'Installation', href: '/docs/installation' },
      { title: 'Quick Start', href: '/docs/quick-start' },
      { title: 'Best Practices', href: '/docs/best-practices' },
    ],
  },
  {
    title: 'Core Concepts',
    href: '/docs/concepts',
    items: [
      { title: 'Architecture', href: '/docs/concepts/architecture' },
      { title: 'Data Sources', href: '/docs/concepts/sources' },
      { title: 'Tools & Packages', href: '/docs/concepts/tools' },
    ],
  },
  {
    title: 'Features',
    href: '/docs/features',
    items: [
      { title: 'Audit & Compliance', href: '/docs/features/audit' },
      { title: 'Post-Pull Hooks', href: '/docs/features/hooks' },
      { title: 'Cache Management', href: '/docs/features/cache' },
      { title: 'Lockfiles', href: '/docs/features/lockfiles' },
    ],
  },
  {
    title: 'CLI Reference',
    href: '/docs/cli',
    items: [
      { title: 'Commands', href: '/docs/cli/commands' },
      { title: 'Configuration', href: '/docs/cli/configuration' },
    ],
  },
  {
    title: 'API Reference',
    href: '/docs/api',
    items: [
      { title: 'REST API', href: '/docs/api/rest' },
      { title: 'Authentication', href: '/docs/api/auth' },
    ],
  },
];

interface DocsSidebarProps {
  onLinkClick?: () => void;
}

export function DocsSidebar({ onLinkClick }: DocsSidebarProps = {}) {
  const pathname = usePathname();

  return (
    <nav className="space-y-6">
      {docsNavigation.map((section) => (
        <div key={section.href}>
          <h4 className="font-semibold text-sm mb-3 text-foreground">
            {section.title}
          </h4>
          {section.items && (
            <ul className="space-y-2">
              {section.items.map((item) => {
                const isActive = pathname === item.href;
                return (
                  <li key={item.href}>
                    <Link
                      href={item.href}
                      onClick={() => {
                        if (onLinkClick) {
                          onLinkClick();
                        }
                      }}
                      className={`block text-sm transition-colors hover:text-foreground ${
                        isActive
                          ? 'text-foreground font-medium'
                          : 'text-muted-foreground'
                      }`}
                    >
                      {item.title}
                    </Link>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      ))}
    </nav>
  );
}
