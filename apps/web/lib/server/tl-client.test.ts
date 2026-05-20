import { selectAuthorizedWorkspaceId } from '../workspace-access';

const memberships = [
  {
    id: 'ws_alpha',
    slug: 'alpha',
  },
  {
    id: 'ws_beta',
    slug: 'beta-team',
  },
];

if (selectAuthorizedWorkspaceId(memberships, null) !== 'ws_alpha') {
  throw new Error('expected missing workspace to resolve to first membership');
}

if (selectAuthorizedWorkspaceId(memberships, 'beta-team') !== 'ws_beta') {
  throw new Error('expected workspace slug to resolve through membership');
}

if (selectAuthorizedWorkspaceId(memberships, 'ws_beta') !== 'ws_beta') {
  throw new Error('expected workspace id to resolve through membership');
}

if (selectAuthorizedWorkspaceId(memberships, 'ws_not_member') !== null) {
  throw new Error('expected non-member workspace to be rejected');
}

if (selectAuthorizedWorkspaceId([], 'alpha') !== null) {
  throw new Error('expected empty membership list to be rejected');
}
