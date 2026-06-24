'use client';

import { createOpenAPIPage } from 'fumadocs-openapi/ui';
import { yamlMediaAdapter } from '@/lib/openapi-media';

export const APIPage = createOpenAPIPage({
  mediaAdapters: {
    'application/yaml': yamlMediaAdapter,
    'text/yaml': yamlMediaAdapter,
  },
});
