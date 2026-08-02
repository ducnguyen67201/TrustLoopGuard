import { describe, expect, it } from 'vitest';

import { readClientOptions } from './client';

describe('readClientOptions', () => {
  it('defaults to the local Rust API URL', () => {
    const options = readClientOptions({ FEATHERLANE_AI_API_KEY: 'tl_live_test' });

    expect(options).toEqual({
      baseUrl: 'http://127.0.0.1:8080',
      apiKey: 'tl_live_test',
    });
  });

  it('trims configured URL and API key', () => {
    const options = readClientOptions({
      FEATHERLANE_AI_URL: ' https://api.example.test/ ',
      FEATHERLANE_AI_API_KEY: ' tl_live_test ',
    });

    expect(options).toEqual({
      baseUrl: 'https://api.example.test/',
      apiKey: 'tl_live_test',
    });
  });

  it('rejects an empty API key without echoing env values', () => {
    expect(() => readClientOptions({ FEATHERLANE_AI_API_KEY: '   ' })).toThrow('FEATHERLANE_AI_API_KEY is required');
  });
});
