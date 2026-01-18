/**
 * Site configuration that reads from environment variables
 */

export const siteConfig = {
  name: process.env.NEXT_PUBLIC_APP_NAME || 'BDP',
  description: 'Blockchain Data Platform - Index, query, and analyze blockchain data',
  url: process.env.NEXT_PUBLIC_APP_URL || 'http://localhost:3000',

  // GitHub configuration
  github: {
    url: process.env.NEXT_PUBLIC_GITHUB_URL || 'https://github.com/datadir-lab/bdp',
    org: process.env.NEXT_PUBLIC_GITHUB_ORG || 'datadir-lab',
    repo: process.env.NEXT_PUBLIC_GITHUB_REPO || 'bdp',
  },

  // Social links
  social: {
    twitter: process.env.NEXT_PUBLIC_TWITTER_URL || 'https://twitter.com/bdp',
    discord: process.env.NEXT_PUBLIC_DISCORD_URL || 'https://discord.gg/bdp',
  },

  // API configuration
  api: {
    url: process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000',
  },

  // Feature flags
  features: {
    analytics: process.env.NEXT_PUBLIC_ENABLE_ANALYTICS === 'true',
    docs: process.env.NEXT_PUBLIC_ENABLE_DOCS !== 'false',
  },
} as const;

// Helper functions
export const getGithubUrl = () => siteConfig.github.url;
export const getGithubIssuesUrl = () => `${siteConfig.github.url}/issues`;
export const getGithubDiscussionsUrl = () => `${siteConfig.github.url}/discussions`;
export const getGithubTreeUrl = (path: string = '') => `${siteConfig.github.url}/tree/main/${path}`;
export const getGithubContributingUrl = () => `${siteConfig.github.url}/blob/main/CONTRIBUTING.md`;
export const getGithubLicenseUrl = () => `${siteConfig.github.url}/blob/main/LICENSE`;
