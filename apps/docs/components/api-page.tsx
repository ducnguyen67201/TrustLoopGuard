import { createAPIPage } from 'fumadocs-openapi/ui';
import { openapi } from '@/lib/openapi';
import { yamlMediaAdapter } from '@/lib/openapi-media';

export const APIPage = createAPIPage(openapi, {
  mediaAdapters: {
    'application/yaml': yamlMediaAdapter,
    'text/yaml': yamlMediaAdapter,
  },
});
