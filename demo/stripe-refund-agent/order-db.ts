import { mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { fileURLToPath } from 'node:url';

import {
  DEMO_CUSTOMER_ID,
  DEMO_ORDER_ID,
  DEMO_PAYMENT_METHOD_ID,
  type OrderRecord,
  type OrderSearchQuery,
} from './types';

interface OrderRow {
  id: string;
  customer_id: string;
  customer_email: string;
  customer_name: string;
  payment_intent_id: string;
  payment_method_id: string;
  payment_method_last4: string;
  amount_paid_minor: number;
  refundable_balance_minor: number;
  currency: string;
  captured: number;
  refund_window_open: number;
  refund_count: number;
}

export interface RefundExecutionRecord {
  orderId: string;
  financialActionId: string;
  amountMinor: number;
  status: string;
  reason: string;
  providerReference?: string;
}

export function orderDatabasePath(): string {
  return (
    process.env.STRIPE_REFUND_AGENT_DB?.trim() ||
    resolve(dirname(fileURLToPath(import.meta.url)), '..', '.data', 'stripe-refund-agent.sqlite')
  );
}

export function resetOrderDatabase(dbPath = orderDatabasePath()): void {
  const db = openDatabase(dbPath);
  try {
    db.exec('DROP TABLE IF EXISTS refunds; DROP TABLE IF EXISTS orders;');
    ensureOrderDatabase(dbPath);
  } finally {
    db.close();
  }
}

export function ensureOrderDatabase(dbPath = orderDatabasePath()): void {
  const db = openDatabase(dbPath);
  try {
    db.exec(`
      CREATE TABLE IF NOT EXISTS orders (
        id TEXT PRIMARY KEY,
        customer_id TEXT NOT NULL,
        customer_email TEXT NOT NULL,
        customer_name TEXT NOT NULL,
        payment_intent_id TEXT NOT NULL,
        payment_method_id TEXT NOT NULL,
        payment_method_last4 TEXT NOT NULL,
        amount_paid_minor INTEGER NOT NULL,
        refundable_balance_minor INTEGER NOT NULL,
        currency TEXT NOT NULL,
        captured INTEGER NOT NULL,
        refund_window_open INTEGER NOT NULL,
        created_at TEXT NOT NULL
      );

      CREATE TABLE IF NOT EXISTS refunds (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        order_id TEXT NOT NULL REFERENCES orders(id),
        financial_action_id TEXT NOT NULL UNIQUE,
        amount_minor INTEGER NOT NULL,
        provider_reference TEXT,
        status TEXT NOT NULL,
        reason TEXT NOT NULL,
        created_at TEXT NOT NULL
      );
    `);

    const paymentIntentId = process.env.STRIPE_PAYMENT_INTENT_ID?.trim() || 'pi_demo_seeded_refund';
    db.prepare(
      `
      INSERT INTO orders (
        id,
        customer_id,
        customer_email,
        customer_name,
        payment_intent_id,
        payment_method_id,
        payment_method_last4,
        amount_paid_minor,
        refundable_balance_minor,
        currency,
        captured,
        refund_window_open,
        created_at
      )
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      ON CONFLICT(id) DO NOTHING
    `,
    ).run(
      DEMO_ORDER_ID,
      DEMO_CUSTOMER_ID,
      'jamie@example.com',
      'Jamie Demo',
      paymentIntentId,
      DEMO_PAYMENT_METHOD_ID,
      '4242',
      10_000,
      10_000,
      'USD',
      1,
      1,
      timestamp(),
    );
  } finally {
    db.close();
  }
}

export function findOrder(query: OrderSearchQuery, dbPath = orderDatabasePath()): OrderRecord | null {
  ensureOrderDatabase(dbPath);
  const db = openDatabase(dbPath);
  try {
    const orderId = query.orderId?.trim() || null;
    const email = query.email?.trim().toLowerCase() || null;
    const last4 = query.last4?.trim() || null;
    const row = db
      .prepare(
        `
        SELECT
          o.*,
          (SELECT COUNT(*) FROM refunds r WHERE r.order_id = o.id AND r.status = 'succeeded') AS refund_count
        FROM orders o
        WHERE (? IS NOT NULL AND lower(o.id) = lower(?))
           OR (? IS NOT NULL AND lower(o.customer_email) = ?)
           OR (? IS NOT NULL AND o.payment_method_last4 = ?)
        LIMIT 1
      `,
      )
      .get(orderId, orderId, email, email, last4, last4) as OrderRow | undefined;
    return row === undefined ? null : rowToOrder(row);
  } finally {
    db.close();
  }
}

export function recordRefundExecution(
  record: RefundExecutionRecord,
  dbPath = orderDatabasePath(),
): boolean {
  ensureOrderDatabase(dbPath);
  const db = openDatabase(dbPath);
  try {
    db.exec('BEGIN IMMEDIATE;');
    const inserted = db
      .prepare(
        `
        INSERT OR IGNORE INTO refunds (
          order_id,
          financial_action_id,
          amount_minor,
          provider_reference,
          status,
          reason,
          created_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?)
      `,
      )
      .run(
        record.orderId,
        record.financialActionId,
        record.amountMinor,
        record.providerReference ?? null,
        record.status,
        record.reason,
        timestamp(),
      );

    if (inserted.changes > 0 && record.status === 'succeeded') {
      db.prepare(
        `
        UPDATE orders
        SET refundable_balance_minor = max(refundable_balance_minor - ?, 0)
        WHERE id = ?
      `,
      ).run(record.amountMinor, record.orderId);
    }

    db.exec('COMMIT;');
    return inserted.changes > 0;
  } catch (error) {
    db.exec('ROLLBACK;');
    throw error;
  } finally {
    db.close();
  }
}

function openDatabase(dbPath: string): DatabaseSync {
  mkdirSync(dirname(dbPath), { recursive: true });
  return new DatabaseSync(dbPath);
}

function rowToOrder(row: OrderRow): OrderRecord {
  return {
    id: row.id,
    customerId: row.customer_id,
    customerEmail: row.customer_email,
    customerName: row.customer_name,
    paymentIntentId: row.payment_intent_id,
    paymentMethodId: row.payment_method_id,
    paymentMethodLast4: row.payment_method_last4,
    amountPaidMinor: row.amount_paid_minor,
    refundableBalanceMinor: row.refundable_balance_minor,
    currency: 'USD',
    captured: row.captured === 1,
    refundWindowOpen: row.refund_window_open === 1,
    refundCount: row.refund_count,
  };
}

function timestamp(): string {
  return '2026-07-06T10:00:00.000Z';
}
