import { Suspense } from 'react';
import { AppLayout } from '@/components/AppLayout';
import { PoliciesView } from '@/components/policies/PoliciesView';

export default function PoliciesPage() {
  return (
    <AppLayout title="Policies">
      <div className="px-4 lg:px-6">
        <Suspense fallback={null}>
          <PoliciesView />
        </Suspense>
      </div>
    </AppLayout>
  );
}
