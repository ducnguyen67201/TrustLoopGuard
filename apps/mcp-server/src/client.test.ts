import { describe, expect, it } from 'vitest';

import { readClientOptions } from './client';

describe('readClientOptions', () => {
  it('defaults to the local Rust API URL', () => {
    const options = readClientOptions({ TLG_API_KEY: 'tl_live_test' });

    expect(options).toEqual({
      baseUrl: 'http://127.0.0.1:8080',
      apiKey: 'tl_live_test',
    });
  });

  it('trims configured URL and API key', () => {
    const options = readClientOptions({
      TLG_URL: ' https://api.example.test/ ',
      TLG_API_KEY: ' tl_live_test ',
    });

    expect(options).toEqual({
      baseUrl: 'https://api.example.test/',
      apiKey: 'tl_live_test',
    });
  });

  it('rejects an empty API key without echoing env values', () => {
    expect(() => readClientOptions({ TLG_API_KEY: '   ' })).toThrow('TLG_API_KEY is required');
  });
});
