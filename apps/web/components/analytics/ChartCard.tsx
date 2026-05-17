"use client";

import type { ReactNode } from 'react';

import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { cn } from '@/lib/utils';

type ChartCardProps = {
  title: string;
  description: string;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
};

export function ChartCard({ title, description, action, children, className }: ChartCardProps) {
  return (
    <Card className={cn(className)}>
      <CardHeader>
        <CardDescription>{description}</CardDescription>
        <CardTitle>{title}</CardTitle>
        {action ? <CardAction>{action}</CardAction> : null}
      </CardHeader>
      <CardContent>{children}</CardContent>
    </Card>
  );
}
