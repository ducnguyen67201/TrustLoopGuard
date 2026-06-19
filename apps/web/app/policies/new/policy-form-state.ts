export type PolicyFieldName =
  | 'policyKey'
  | 'description'
  | 'severity'
  | 'action'
  | 'agentId'
  | 'sourceYaml';

export type PolicyFormState = {
  formError?: string;
  fieldErrors?: Partial<Record<PolicyFieldName, string>>;
};

export type CreatePolicyAction = (
  state: PolicyFormState,
  formData: FormData,
) => Promise<PolicyFormState>;
