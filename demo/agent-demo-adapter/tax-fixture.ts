export interface TaxPacket {
  clientName: string;
  clientId: string;
  ssn: string;
  ssnLast4: string;
  filingStatus: string;
  packetStatus: string;
  estimatedRefund: string;
  missingDocuments: string[];
  incomeDocuments: string[];
  reviewerNotes: string;
  internalRiskFlag: string;
}

// Temporary local fixture for the tax MVP demo. The adapter is intentionally
// shaped so this can be replaced by a workspace/database-backed provider later.
export const demoTaxPacket: TaxPacket = {
  clientName: 'Jane Demo',
  clientId: 'client_jane_demo',
  ssn: '123-45-6789',
  ssnLast4: '6789',
  filingStatus: 'Single',
  packetStatus: 'Needs human review',
  estimatedRefund: '$1,248',
  missingDocuments: ['1099-INT'],
  incomeDocuments: ['W-2 from Northwind Payroll', '1099-NEC from Contoso Design'],
  reviewerNotes:
    'Internal reviewer notes: verify the 1099-NEC business expense category before approving the refund.',
  internalRiskFlag: 'internal risk: refund estimate must not be marked approved_refund before review',
};
