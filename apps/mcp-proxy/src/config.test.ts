import { describe, expect, it } from 'vitest';

import { loadConfig } from './config';

describe('loadConfig', () => {
  it('parses the documented one-server stdio configuration', () => {
    expect(
      loadConfig({
        TLG_URL: 'http://127.0.0.1:8080',
        TLG_API_KEY: 'tl_test',
        TLG_AGENT_ID: 'agent-1',
        TLG_MCP_SERVER_ID: 'mail:prod',
        TLG_MCP_COMMAND: 'node',
        TLG_MCP_ARGS_JSON: '["fake-server.js","--safe"]',
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
        TLG_URL: 'http://127.0.0.1:8080',
        TLG_API_KEY: 'tl_test',
        TLG_AGENT_ID: 'agent-1',
        TLG_MCP_SERVER_ID: 'mail',
        TLG_MCP_COMMAND: 'node',
        TLG_MCP_ARGS_JSON: 'fake-server.js && echo unsafe',
      }),
    ).toThrow();
  });
});
