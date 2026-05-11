# Authenticating to the guard server

The guard server (`/v1/check` and the agent registry) accepts two kinds of bearer token:

- The static `TL_API_KEY` set on the server at boot. This is the legacy bearer used by CI, examples, and the quickstart.
- Any non-revoked personal API key minted from the dashboard.

Both formats use the same `Authorization: Bearer <token>` header — pick whichever the deployment exposes.

## Using a personal API key

After signing in, the **API keys** page mints bearer tokens you can use with the SDK:

```bash
curl -X POST $TL_SERVER_URL/v1/check \
  -H "Authorization: Bearer tlg_..." \
  -H "Content-Type: application/json" \
  -d '{
        "agent_id": "support-bot",
        "channel": "chat",
        "input": "...",
        "proposed_output": "..."
      }'
```

Revocation propagates within roughly one minute. For immediate cut-off, restart the guard server so the in-memory key cache is dropped.
