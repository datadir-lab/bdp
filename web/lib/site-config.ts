// Use window.location.origin in browser, env vars for SSR
const getBaseUrl = () => {
  if (typeof window !== 'undefined') {
    return window.location.origin;
  }
  return process.env.NEXT_PUBLIC_APP_URL || 'http://localhost:3000';
};

export const siteConfig = {
  name: process.env.NEXT_PUBLIC_APP_NAME || 'BDP',
  get url() { return getBaseUrl(); },
  get apiUrl() { return getBaseUrl(); },
  contactEmail: process.env.NEXT_PUBLIC_CONTACT_EMAIL || 'sebastian.stupak@pm.me',
  github: {
    url: process.env.NEXT_PUBLIC_GITHUB_URL || 'https://github.com/datadir-lab/bdp',
    org: process.env.NEXT_PUBLIC_GITHUB_ORG || 'datadir-lab',
    repo: process.env.NEXT_PUBLIC_GITHUB_REPO || 'bdp',
  },
  social: {
    twitter: process.env.NEXT_PUBLIC_TWITTER_URL,
    discord: process.env.NEXT_PUBLIC_DISCORD_URL,
  },
  install: {
    unix: 'curl -sSfL https://bdp.dev/install.sh | sh',
    windows: 'irm https://bdp.dev/install.ps1 | iex',
  },
};

export function getGithubIssuesUrl(): string {
  return `${siteConfig.github.url}/issues`;
}

export function getGithubDiscussionsUrl(): string {
  return `${siteConfig.github.url}/discussions`;
}

export function getGithubContributingUrl(): string {
  return `${siteConfig.github.url}/blob/main/CONTRIBUTING.md`;
}

export function getGithubLicenseUrl(): string {
  return `${siteConfig.github.url}/blob/main/LICENSE`;
}

export function getGithubTranslationUrl(): string {
  return `${siteConfig.github.url}/tree/main/web/app/[locale]/docs/content`;
}
