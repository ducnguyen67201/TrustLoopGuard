import defaultMdxComponents from 'fumadocs-ui/mdx';
import type { MDXComponents } from 'mdx/types';
import { APIPage } from './api-page';

export const mdxComponents = {
  ...defaultMdxComponents,
  APIPage,
} satisfies MDXComponents;
