'use client';

import { IconFileCertificate, IconReceipt, IconShieldCheck } from '@tabler/icons-react';
import Link from 'next/link';

import { Badge } from '@/components/ui/badge';

type FinancialAuthorizationModelProps = {
  active: 'policies' | 'mandates' | 'actions';
  contextQuery: string;
};

const MODEL_STEPS = [
  {
    id: 'policies',
    icon: <IconShieldCheck />,
    title: 'Policies',
    detail: 'Standing controls',
    cadence: 'Once',
    href: '/policies',
  },
  {
    id: 'mandates',
    icon: <IconFileCertificate />,
    title: 'Mandates',
    detail: 'User intent boundary',
    cadence: 'Per task',
    href: '/financial/mandates',
  },
  {
    id: 'actions',
    icon: <IconReceipt />,
    title: 'Actions',
    detail: 'Runtime payment proof',
    cadence: 'Per payment',
    href: '/financial',
  },
] as const;

export function FinancialAuthorizationModel({
  active,
  contextQuery,
}: FinancialAuthorizationModelProps) {
  return (
    <div className="grid gap-3 rounded-lg border bg-card p-3">
      <div className="flex flex-col gap-1 md:flex-row md:items-center md:justify-between">
        <p className="text-sm font-medium">Authorization path</p>
        <p className="text-sm text-muted-foreground">
          Payment signs only when action fits policy + mandate.
        </p>
      </div>
      <div className="grid gap-2 md:grid-cols-3">
        {MODEL_STEPS.map((step) => {
          const isActive = step.id === active;
          return (
            <Link
              key={step.id}
              href={`${step.href}${contextQuery}`}
              className="grid min-w-0 gap-2 rounded-md border p-3 transition-colors hover:bg-muted/40"
              data-active={isActive ? 'true' : 'false'}
            >
              <div className="flex items-center justify-between gap-3">
                <span className="text-muted-foreground [&>svg]:size-4">{step.icon}</span>
                <Badge variant={isActive ? 'allow' : 'outline'}>
                  {isActive ? 'Here' : step.cadence}
                </Badge>
              </div>
              <div className="grid min-w-0 gap-1">
                <span className="truncate text-sm font-medium">{step.title}</span>
                <p className="text-sm text-muted-foreground">{step.detail}</p>
              </div>
            </Link>
          );
        })}
      </div>
    </div>
  );
}
