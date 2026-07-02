const STEPS = [
  'Create your workspace',
  'Connect your agent',
  'See your first decision',
] as const;

/**
 * The shared 3-segment progress rail for the first-run onboarding flow
 * (/onboarding/workspace → /onboarding/connect → /onboarding/verify).
 */
export function OnboardingProgress({ current }: { current: 1 | 2 | 3 }) {
  return (
    <div className="grid gap-2">
      <p className="text-3xs font-medium uppercase tracking-label text-muted-foreground">
        Step {current} of {STEPS.length} · {STEPS[current - 1]}
      </p>
      <ol
        className="flex items-center gap-2"
        aria-label={`Setup progress: step ${current} of ${STEPS.length}`}
      >
        {STEPS.map((label, index) => (
          <li
            key={label}
            aria-current={index + 1 === current ? 'step' : undefined}
            className={`h-1.5 flex-1 rounded-full ${index + 1 <= current ? 'bg-primary' : 'bg-border'}`}
            title={`Step ${index + 1} — ${label}`}
          />
        ))}
      </ol>
    </div>
  );
}
