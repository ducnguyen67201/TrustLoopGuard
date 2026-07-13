import { mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { DatabaseSync, type SQLOutputValue } from 'node:sqlite';
import { fileURLToPath } from 'node:url';

import {
  DEMO_CUSTOMER_ID,
  DEMO_ORDER_ID,
  DEMO_PAYMENT_METHOD_ID,
  type OrderRecord,
  type OrderSearchQuery,
  type CustomerBackendState,
  type RefundRecord,
} from './types';

type SqlRow = Record<string, SQLOutputValue>;

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

export function resetOrderDatabase(
  dbPath = orderDatabasePath(),
  paymentIntentId = seededPaymentIntentId(),
): void {
  const db = openDatabase(dbPath);
  try {
    db.exec('DROP TABLE IF EXISTS refunds; DROP TABLE IF EXISTS orders;');
  } finally {
    db.close();
  }
  ensureOrderDatabase(dbPath, paymentIntentId);
}

export function ensureOrderDatabase(
  dbPath = orderDatabasePath(),
  paymentIntentId = seededPaymentIntentId(),
): void {
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
      .get(orderId, orderId, email, email, last4, last4);
    return row === undefined ? null : rowToOrder(row);
  } finally {
    db.close();
  }
}

export function customerBackendState(dbPath = orderDatabasePath()): CustomerBackendState {
  ensureOrderDatabase(dbPath);
  return {
    orders: listOrders(dbPath),
    refunds: listRefunds(dbPath),
  };
}

export function listOrders(dbPath = orderDatabasePath()): OrderRecord[] {
  ensureOrderDatabase(dbPath);
  const db = openDatabase(dbPath);
  try {
    const rows = db
      .prepare(
        `
        SELECT
          o.*,
          (SELECT COUNT(*) FROM refunds r WHERE r.order_id = o.id AND r.status = 'succeeded') AS refund_count
        FROM orders o
        ORDER BY o.created_at DESC, o.id ASC
      `,
      )
      .all();
    return rows.map(rowToOrder);
  } finally {
    db.close();
  }
}

export function listRefunds(dbPath = orderDatabasePath()): RefundRecord[] {
  ensureOrderDatabase(dbPath);
  const db = openDatabase(dbPath);
  try {
    const rows = db
      .prepare(
        `
        SELECT
          id,
          order_id,
          financial_action_id,
          amount_minor,
          provider_reference,
          status,
          reason,
          created_at
        FROM refunds
        ORDER BY id DESC
      `,
      )
      .all();
    return rows.map(rowToRefund);
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

function rowToOrder(row: SqlRow): OrderRecord {
  return {
    id: textValue(row, 'id'),
    customerId: textValue(row, 'customer_id'),
    customerEmail: textValue(row, 'customer_email'),
    customerName: textValue(row, 'customer_name'),
    paymentIntentId: textValue(row, 'payment_intent_id'),
    paymentMethodId: textValue(row, 'payment_method_id'),
    paymentMethodLast4: textValue(row, 'payment_method_last4'),
    amountPaidMinor: numberValue(row, 'amount_paid_minor'),
    refundableBalanceMinor: numberValue(row, 'refundable_balance_minor'),
    currency: 'USD',
    captured: numberValue(row, 'captured') === 1,
    refundWindowOpen: numberValue(row, 'refund_window_open') === 1,
    refundCount: numberValue(row, 'refund_count'),
  };
}

function rowToRefund(row: SqlRow): RefundRecord {
  const providerReference = nullableTextValue(row, 'provider_reference');
  return {
    id: numberValue(row, 'id'),
    orderId: textValue(row, 'order_id'),
    financialActionId: textValue(row, 'financial_action_id'),
    amountMinor: numberValue(row, 'amount_minor'),
    providerReference,
    status: textValue(row, 'status'),
    reason: textValue(row, 'reason'),
    createdAt: textValue(row, 'created_at'),
  };
}

function textValue(row: SqlRow, key: string): string {
  const value = row[key];
  if (typeof value !== 'string') throw new Error(`SQLite column ${key} is not text`);
  return value;
}

function nullableTextValue(row: SqlRow, key: string): string | undefined {
  const value = row[key];
  if (value === null) return undefined;
  if (typeof value !== 'string') throw new Error(`SQLite column ${key} is not text`);
  return value;
}

function numberValue(row: SqlRow, key: string): number {
  const value = row[key];
  if (typeof value !== 'number') throw new Error(`SQLite column ${key} is not numeric`);
  return value;
}

function timestamp(): string {
  return '2026-07-06T10:00:00.000Z';
}

function seededPaymentIntentId(): string {
  return process.env.STRIPE_PAYMENT_INTENT_ID?.trim() || 'pi_demo_seeded_refund';
}
