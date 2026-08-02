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
      src="/featherlane-ai-logo.svg"
      alt={alt}
      width={40}
      height={40}
      priority={priority}
      className={cn('size-8 rounded-full', className)}
    />
  );
}
