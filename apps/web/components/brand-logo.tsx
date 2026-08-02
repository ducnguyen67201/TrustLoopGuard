import Image from 'next/image';

import { cn } from '@/lib/utils';

interface BrandLogoProps {
  alt?: string;
  className?: string;
  priority?: boolean;
}

export function BrandLogo({
  alt = 'Featherlane AI',
  className,
  priority = false,
}: BrandLogoProps) {
  return (
    <Image
      src="/featherlane-ai-logo.png"
      alt={alt}
      width={44}
      height={41}
      priority={priority}
      className={cn('h-8 w-auto object-contain', className)}
    />
  );
}
