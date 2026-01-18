import { useTranslations } from 'next-intl';
import { Link } from '@/i18n/navigation';
import { Button } from '@/components/ui/button';
import { FileQuestion, Home, Search } from 'lucide-react';

// This 404 page is rendered within the [locale] layout
// which includes Header, Footer, ThemeProvider, and i18n context
export default function NotFound() {
  const t = useTranslations('notFound');

  return (
    <div className="container flex min-h-[calc(100vh-200px)] items-center justify-center">
      <div className="mx-auto max-w-md text-center space-y-6 py-20">
        <div className="flex justify-center">
          <FileQuestion className="h-24 w-24 text-muted-foreground" />
        </div>

        <div className="space-y-2">
          <h1 className="text-4xl font-bold tracking-tight">{t('title')}</h1>
          <h2 className="text-2xl font-semibold">{t('heading')}</h2>
          <p className="text-muted-foreground">
            {t('description')}
          </p>
        </div>

        <div className="flex flex-col sm:flex-row gap-3 justify-center pt-4">
          <Button asChild>
            <Link href="/">
              <Home className="mr-2 h-4 w-4" />
              {t('goHome')}
            </Link>
          </Button>
          <Button variant="outline" asChild>
            <Link href="/search">
              <Search className="mr-2 h-4 w-4" />
              {t('search')}
            </Link>
          </Button>
        </div>
      </div>
    </div>
  );
}
