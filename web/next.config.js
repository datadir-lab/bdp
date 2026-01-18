const withNextIntl = require('next-intl/plugin')('./i18n/request.ts');
const createMDX = require('@next/mdx');

/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  poweredByHeader: false,

  // MDX support
  pageExtensions: ['js', 'jsx', 'md', 'mdx', 'ts', 'tsx'],

  // Output standalone for Docker optimization
  // Note: For Pagefind, we'll pre-render docs pages
  output: 'standalone',


  // Environment variables
  env: {
    NEXT_PUBLIC_API_URL: process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000',
  },

  // Image optimization
  images: {
    remotePatterns: [
      {
        protocol: 'http',
        hostname: 'localhost',
        port: '',
        pathname: '/**',
      },
    ],
    formats: ['image/avif', 'image/webp'],
  },

  // Note: Custom headers not supported with static export
  // Security headers should be configured in hosting provider (Vercel, Netlify, etc.)

  // Webpack configuration
  webpack: (config) => {
    config.resolve.fallback = { fs: false, path: false };
    return config;
  },
};

// MDX configuration - basic setup for Turbopack
const withMDX = createMDX({
  extension: /\.mdx?$/,
});

// Compose plugins: MDX first, then next-intl
module.exports = withNextIntl(withMDX(nextConfig));
