import { orderDatabasePath, resetOrderDatabase } from './order-db';
import { DEMO_ORDER_ID } from './types';

resetOrderDatabase();

process.stdout.write(`SQLite refund demo DB ready: ${orderDatabasePath()}\n`);
process.stdout.write(`Seeded order: ${DEMO_ORDER_ID}\n`);
