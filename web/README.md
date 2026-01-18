# BDP Web Interface

Next.js-based web interface for the Biological Data Platform (BDP).

## Tech Stack

- **Next.js 16** - React framework with App Router
- **TypeScript** - Type-safe development
- **Tailwind CSS** - Utility-first CSS framework
- **Radix UI** - Accessible component primitives
- **Nextra** - Documentation framework
- **Lucide React** - Icon library

## Project Structure

```
web/
├── app/                    # Next.js App Router
│   ├── layout.tsx         # Root layout
│   └── page.tsx           # Homepage
├── components/            # React components
│   ├── ui/               # UI components (buttons, etc.)
│   └── layout/           # Layout components (header, footer)
├── lib/                   # Utility functions
│   ├── api-client.ts     # API client wrapper
│   ├── types.ts          # TypeScript types
│   └── utils.ts          # Utility functions
├── pages/                 # Nextra documentation pages
│   └── docs/             # Documentation content
├── styles/               # Global styles
│   └── globals.css       # Global CSS with Tailwind
├── next.config.js        # Next.js configuration
├── tailwind.config.js    # Tailwind configuration
├── tsconfig.json         # TypeScript configuration
└── theme.config.tsx      # Nextra theme configuration
```

## Features

- **Modern UI** - Built with Tailwind CSS and Radix UI
- **Type Safety** - Full TypeScript support
- **API Integration** - Pre-configured API client
- **Documentation** - Integrated Nextra docs
- **Responsive Design** - Mobile-first approach
- **Accessibility** - WCAG compliant components
- **Performance** - Optimized with Next.js 16
- **Data Visualization** - Protein and gene data visualization
- **Search Interface** - Advanced search and filtering capabilities

## API Client Usage

```typescript
import { apiClient } from '@/lib/api-client';

// GET request - fetch protein data
const { data } = await apiClient.get('/proteins/P12345');

// POST request - submit query
const { data } = await apiClient.post('/search', {
  query: 'kinase',
  organism: 'human'
});

// Paginated request
const { data } = await apiClient.getPaginated('/proteins', {
  page: 1,
  limit: 20
});
```

## Environment Variables

Create a `.env.local` file:

```env
NEXT_PUBLIC_API_URL=http://localhost:8080
NEXT_PUBLIC_APP_NAME=BDP
NEXT_PUBLIC_APP_URL=http://localhost:3000
```

## Documentation

The documentation is powered by Nextra and can be accessed at `/docs` when running the development server.

To add new documentation pages:

1. Create MDX files in `pages/docs/`
2. Update `pages/docs/_meta.json` to add to navigation
3. Write content using MDX (Markdown + JSX)

## Deployment

### Vercel (Recommended)

```bash
# Install Vercel CLI
npm i -g vercel

# Deploy
vercel
```

### Docker

```bash
# Build image
docker build -t bdp-web .

# Run container
docker run -p 3000:3000 bdp-web
```

### Static Export

```bash
# Build static files
npm run build

# Files will be in the 'out' directory
```

## Contributing

Please read the [Contributing Guide](../docs/CONTRIBUTING.md) for details on our code of conduct and development process.

## License

This project is licensed under the MIT License - see the [LICENSE](../LICENSE) file for details.
