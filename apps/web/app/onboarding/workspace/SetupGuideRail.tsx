import type { ComponentType } from 'react';
import {
  IconBuilding,
  IconKey,
  IconShieldCheck,
  IconUsers,
} from '@tabler/icons-react';

interface GuideItem {
  icon: ComponentType<{ className?: string }>;
  title: string;
  description: string;
}

const GUIDE_ITEMS: readonly GuideItem[] = [
  {
    icon: IconShieldCheck,
    title: 'Safety rules',
    description: 'You decide what your AI can and can’t do. We watch every request.',
  },
  {
    icon: IconBuilding,
    title: 'Your AI assistants',
    description: 'Each bot or app you connect lives here, so you can see how it behaves.',
  },
  {
    icon: IconUsers,
    title: 'Your team',
    description: 'Invite people to help — choose who can change rules and who just watches.',
  },
  {
    icon: IconKey,
    title: 'A connection key',
    description: 'One secret key links your app to this workspace. We’ll create it for you.',
  },
];

/**
 * Skimmable summary of what a workspace holds. Rendered as a guide rail beside
 * the setup form so a non-technical user understands the shape of the product
 * in plain words before committing to the first step. Order is illustrative,
 * not a sequence, so it renders as an unordered list.
 */
export function SetupGuideRail() {
  return (
    <ul className="grid gap-4">
      {GUIDE_ITEMS.map((item) => (
        <li key={item.title} className="grid grid-cols-[auto_minmax(0,1fr)] items-start gap-x-3">
          <span
            aria-hidden
            className="flex size-8 shrink-0 items-center justify-center rounded-lg border border-border bg-card text-muted-foreground"
          >
            <item.icon className="size-4" />
          </span>
          <div>
            <h3 className="text-sm font-semibold text-foreground">{item.title}</h3>
            <p className="mt-0.5 text-sm leading-6 text-muted-foreground">
              {item.description}
            </p>
          </div>
        </li>
      ))}
    </ul>
  );
}
