import 'server-only';

import { cache } from 'react';
import postgres from 'postgres';

import {
  demoCategorySchema,
  demoSlugSchema,
  isActiveDemoProfile,
  parseOutboundDemoProfile,
  type DemoCategory,
  type JsonValue,
  type OutboundDemoProfile,
} from '../../app/demo/company-profile';
import { env } from '../../env';

type DemoProfileRow = {
  profile: JsonValue;
};

let database: ReturnType<typeof postgres> | undefined;

function getDatabase(): ReturnType<typeof postgres> | null {
  if (!env.OUTBOUND_DEMO_DATABASE_URL) {
    return null;
  }

  database ??= postgres(env.OUTBOUND_DEMO_DATABASE_URL, {
    max: 2,
    idle_timeout: 20,
    connect_timeout: 10,
    prepare: false,
    connection: {
      application_name: 'trustloop-marketing-demo-reader',
      default_transaction_read_only: true,
      statement_timeout: 3_000,
    },
  });
  return database;
}

async function readActiveDemoProfile(
  category: DemoCategory,
  slug: string,
): Promise<OutboundDemoProfile | null> {
  const parsedCategory = demoCategorySchema.safeParse(category);
  const parsedSlug = demoSlugSchema.safeParse(slug);
  const sql = getDatabase();
  if (!parsedCategory.success || !parsedSlug.success || !sql) {
    return null;
  }

  try {
    const rows = await sql<DemoProfileRow[]>`
      SELECT profile
      FROM outbound_demo_profiles
      WHERE category = ${parsedCategory.data}
        AND slug = ${parsedSlug.data}
        AND status = 'active'
        AND live_verified = TRUE
        AND (expires_at IS NULL OR expires_at > NOW())
      LIMIT 1
    `;
    const profile = rows[0] ? parseOutboundDemoProfile(rows[0].profile) : null;
    return profile &&
      profile.category === parsedCategory.data &&
      profile.slug === parsedSlug.data &&
      isActiveDemoProfile(profile)
      ? profile
      : null;
  } catch {
    console.error('Unable to read an outbound demo profile.');
    return null;
  }
}

export const getActiveDemoProfile = cache(readActiveDemoProfile);

async function readActiveDemoProfileBySlug(slug: string): Promise<OutboundDemoProfile | null> {
  const parsedSlug = demoSlugSchema.safeParse(slug);
  const sql = getDatabase();
  if (!parsedSlug.success || !sql) {
    return null;
  }

  try {
    const rows = await sql<DemoProfileRow[]>`
      SELECT profile
      FROM outbound_demo_profiles
      WHERE slug = ${parsedSlug.data}
        AND status = 'active'
        AND live_verified = TRUE
        AND (expires_at IS NULL OR expires_at > NOW())
      LIMIT 2
    `;
    if (rows.length !== 1) {
      return null;
    }
    const profile = rows[0] ? parseOutboundDemoProfile(rows[0].profile) : null;
    return profile && profile.slug === parsedSlug.data && isActiveDemoProfile(profile)
      ? profile
      : null;
  } catch {
    console.error('Unable to read an outbound demo profile.');
    return null;
  }
}

export const getActiveDemoProfileBySlug = cache(readActiveDemoProfileBySlug);
