// Root layout required by Next.js when app/not-found.tsx exists
// This just passes through children - actual layout is in [locale]/layout.tsx
export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return children;
}
