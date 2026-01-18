// Root-level not-found page - catches requests outside locale routing
// Uses standalone Header/Footer components (same design, no i18n context)

import { Inter, JetBrains_Mono } from 'next/font/google';
import { NotFoundContent } from './not-found-content';
import '@/styles/globals.css';

const inter = Inter({ subsets: ['latin'], variable: '--font-inter' });
const jetbrainsMono = JetBrains_Mono({
  subsets: ['latin'],
  variable: '--font-mono',
  display: 'swap',
});

export default function RootNotFound() {
  return (
    <html lang="en" suppressHydrationWarning className={`${inter.variable} ${jetbrainsMono.variable}`}>
      <body className="font-sans">
        <NotFoundContent />
      </body>
    </html>
  );
}
