export interface DemoMeetingPromptState {
  completedInteractions: number;
  isOpen: boolean;
  hasBeenShown: boolean;
}

export type DemoMeetingPromptAction = { type: 'interaction_completed' } | { type: 'dismissed' };

export const initialDemoMeetingPromptState: DemoMeetingPromptState = {
  completedInteractions: 0,
  isOpen: false,
  hasBeenShown: false,
};

export function reduceDemoMeetingPromptState(
  state: DemoMeetingPromptState,
  action: DemoMeetingPromptAction,
): DemoMeetingPromptState {
  if (action.type === 'dismissed') {
    return { ...state, isOpen: false };
  }

  const completedInteractions = state.completedInteractions + 1;
  const shouldOpen = completedInteractions === 2 && !state.hasBeenShown;

  return {
    completedInteractions,
    isOpen: shouldOpen,
    hasBeenShown: state.hasBeenShown || shouldOpen,
  };
}
