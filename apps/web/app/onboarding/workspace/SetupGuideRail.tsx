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
    icon: IconBuilding,
    title: 'Workspace',
    description: 'One guarded AI product, app, bot, or customer deployment.',
  },
  {
    icon: IconShieldCheck,
    title: 'Policies',
    description: 'Rules that block, rewrite, escalate, or allow what your agent does.',
  },
  {
    icon: IconUsers,
    title: 'Team',
    description: 'Invite teammates with owner, admin, editor, or viewer access.',
  },
  {
    icon: IconKey,
    title: 'API keys',
    description: 'Runtime keys that connect your app to this workspace’s guardrails.',
  },
];

/**
 * Ordered walkthrough of everything a workspace will hold. Rendered as a guide
 * rail beside the setup form so the user understands the shape of the product
 * before committing to the first step.
 */
export function SetupGuideRail() {
  return (
    <ol className="grid gap-0">
      {GUIDE_ITEMS.map((item, index) => {
        const isLast = index === GUIDE_ITEMS.length - 1;
        return (
          <li key={item.title} className="grid grid-cols-[auto_minmax(0,1fr)] gap-x-4">
            <div className="flex flex-col items-center">
              <span
                aria-hidden
                className="flex size-9 shrink-0 items-center justify-center rounded-lg border border-border bg-card text-muted-foreground"
              >
                <item.icon className="size-4" />
              </span>
              {!isLast ? (
                <span aria-hidden className="my-1 w-px flex-1 bg-border" />
              ) : null}
            </div>
            <div className={isLast ? 'pb-0' : 'pb-6'}>
              <div className="flex items-baseline gap-2">
                <h3 className="text-sm font-semibold text-foreground">{item.title}</h3>
                <span className="font-mono text-xs tabular-nums text-muted-foreground">
                  {String(index + 1).padStart(2, '0')}
                </span>
              </div>
              <p className="mt-1 text-sm leading-6 text-muted-foreground">
                {item.description}
              </p>
            </div>
          </li>
        );
      })}
    </ol>
  );
}
