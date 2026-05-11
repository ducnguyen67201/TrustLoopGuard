# Authentication

TrustLoopGuard's dashboard supports two sign-in methods, configured independently:

- **Email and password** — open self-signup. Designed for local self-hosting.
- **Google OAuth** — single sign-on. Recommended for shared deployments.

You can enable either, both, or neither. If neither is configured, the sign-in page renders a configuration message instead of a login form.

## Local Docker

For a single-developer local instance:

```env
AUTH_SECRET=<openssl rand -base64 32>
DATABASE_URL=postgres://trustloop:trustloop@localhost:5432/trustloop
AUTH_ALLOW_SIGNUP=true
```

Visit `/signup` to create an account.

## Shared deployment with Google OAuth

For team or production use:

```env
AUTH_SECRET=<long random secret, kept private>
DATABASE_URL=postgres://...
AUTH_TRUST_HOST=true

AUTH_GOOGLE_ID=<from Google Cloud Console>
AUTH_GOOGLE_SECRET=<from Google Cloud Console>

# Optional: disable open signup so only Google users can access.
AUTH_ALLOW_SIGNUP=false
```

### Creating the Google OAuth client

1. Open the [Google Cloud Console credentials page](https://console.cloud.google.com/apis/credentials).
2. Create an OAuth 2.0 Client ID of type **Web application**.
3. Add an authorized redirect URI: `https://<your-domain>/api/auth/callback/google`.
4. Copy the client ID and secret into `AUTH_GOOGLE_ID` and `AUTH_GOOGLE_SECRET`.

### Verified emails only

Sign-ins from Google accounts whose email is not verified by Google are rejected. Users with the same email across email/password and Google sign in to a single account — the Google identity is linked on first successful sign-in.

## Environment variable reference

| Variable | Required | Purpose |
| --- | --- | --- |
| `DATABASE_URL` | yes | Postgres connection string |
| `AUTH_SECRET` | yes | Signs session JWTs; at least 32 random bytes |
| `AUTH_ALLOW_SIGNUP` | no (default `true`) | When `false`, hides `/signup` and the email/password form |
| `AUTH_GOOGLE_ID` | no | Enables Google sign-in when set together with the secret |
| `AUTH_GOOGLE_SECRET` | no | Pairs with `AUTH_GOOGLE_ID` |
| `AUTH_TRUST_HOST` | no (default `false`) | Set to `true` behind a reverse proxy or hosted environment |
