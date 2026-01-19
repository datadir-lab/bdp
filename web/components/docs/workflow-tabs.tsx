'use client';

import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';

interface WorkflowTabsProps {
  children: React.ReactNode;
  defaultValue?: string;
}

export function WorkflowTabs({ children, defaultValue = 'protein-analysis' }: WorkflowTabsProps) {
  return (
    <Tabs defaultValue={defaultValue} className="w-full">
      {children}
    </Tabs>
  );
}

export function WorkflowTabsList({ children }: { children: React.ReactNode }) {
  return <TabsList className="grid w-full grid-cols-2 md:grid-cols-4">{children}</TabsList>;
}

export function WorkflowTabsTrigger({ value, children }: { value: string; children: React.ReactNode }) {
  return <TabsTrigger value={value}>{children}</TabsTrigger>;
}

export function WorkflowTabsContent({ value, children }: { value: string; children: React.ReactNode }) {
  return <TabsContent value={value}>{children}</TabsContent>;
}
