import { describe, expect, it } from 'vitest';

import { loadConfig } from './config';

describe('loadConfig', () => {
  it('parses the documented one-server stdio configuration', () => {
    expect(
      loadConfig({
        FEATHERLANE_AI_URL: 'http://127.0.0.1:8080',
        FEATHERLANE_AI_API_KEY: 'tl_test',
        FEATHERLANE_AI_AGENT_ID: 'agent-1',
        FEATHERLANE_AI_MCP_SERVER_ID: 'mail:prod',
        FEATHERLANE_AI_MCP_COMMAND: 'node',
        FEATHERLANE_AI_MCP_ARGS_JSON: '["fake-server.js","--safe"]',
      }),
    ).toEqual({
      baseUrl: 'http://127.0.0.1:8080',
      apiKey: 'tl_test',
      agentId: 'agent-1',
      serverId: 'mail:prod',
      command: 'node',
      args: ['fake-server.js', '--safe'],
    });
  });

  it('rejects shell-shaped args instead of parsing a command line', () => {
    expect(() =>
      loadConfig({
        FEATHERLANE_AI_URL: 'http://127.0.0.1:8080',
        FEATHERLANE_AI_API_KEY: 'tl_test',
        FEATHERLANE_AI_AGENT_ID: 'agent-1',
        FEATHERLANE_AI_MCP_SERVER_ID: 'mail',
        FEATHERLANE_AI_MCP_COMMAND: 'node',
        FEATHERLANE_AI_MCP_ARGS_JSON: 'fake-server.js && echo unsafe',
      }),
    ).toThrow();
  });
});
