import assert from 'node:assert/strict';
import test from 'node:test';

import {
  initialDemoMeetingPromptState,
  reduceDemoMeetingPromptState,
} from './demo-meeting-prompt-state';

test('opens the meeting prompt after the second completed demo interaction', () => {
  const afterFirst = reduceDemoMeetingPromptState(initialDemoMeetingPromptState, {
    type: 'interaction_completed',
  });
  const afterSecond = reduceDemoMeetingPromptState(afterFirst, {
    type: 'interaction_completed',
  });

  assert.deepEqual(afterFirst, {
    completedInteractions: 1,
    isOpen: false,
    hasBeenShown: false,
  });
  assert.deepEqual(afterSecond, {
    completedInteractions: 2,
    isOpen: true,
    hasBeenShown: true,
  });
});

test('does not reopen the meeting prompt after it is dismissed', () => {
  const afterFirst = reduceDemoMeetingPromptState(initialDemoMeetingPromptState, {
    type: 'interaction_completed',
  });
  const afterSecond = reduceDemoMeetingPromptState(afterFirst, {
    type: 'interaction_completed',
  });
  const dismissed = reduceDemoMeetingPromptState(afterSecond, { type: 'dismissed' });
  const afterThird = reduceDemoMeetingPromptState(dismissed, {
    type: 'interaction_completed',
  });

  assert.equal(dismissed.isOpen, false);
  assert.deepEqual(afterThird, {
    completedInteractions: 3,
    isOpen: false,
    hasBeenShown: true,
  });
});
