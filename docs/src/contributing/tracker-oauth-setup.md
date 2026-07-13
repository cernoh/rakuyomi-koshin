# Setting Up Tracker OAuth for Development

RakuYomi integrates with AniList and MyAnimeList for reading progress tracking. This guide explains how to set up OAuth credentials for development and testing.

## Overview

Both AniList and MAL use OAuth 2.0 for authentication. RakuYomi uses **public OAuth clients** (no client secret required):

- **AniList**: Implicit grant flow (`response_type=token`)
- **MyAnimeList**: PKCE flow (`response_type=code` with `code_challenge`)

The client IDs are hardcoded in the backend:
- AniList: `16329`
- MAL: `c46c9e24640a64dad5be5ca7a1a53a0f`

For most development, you can use these existing client IDs. However, if you're working on the auth flow itself or hitting rate limits, you may want to register your own OAuth apps.

## Registering Your Own OAuth Apps

### AniList

1. Go to [AniList Developer Settings](https://anilist.co/settings/developer)
2. Click "Create New Client"
3. Fill in the form:
   - **Name**: `RakuYomi Dev` (or any name)
   - **Redirect URL**: `https://anilist.co/api/v2/oauth/pin` (AniList's PIN redirect for implicit grant)
   - **Description**: Optional, e.g., "Development version of RakuYomi"
4. Click "Save"
5. Note the **Client ID** from the created app

### MyAnimeList

1. Go to [MAL API Settings](https://myanimelist.net/apiconfig)
2. Click "Create ID"
3. Fill in the form:
   - **App Name**: `RakuYomi Dev`
   - **App URL**: `https://github.com/yourusername/rakuyomi` (or any URL)
   - **App Redirect URL**: Leave empty (MAL PKCE doesn't require it)
   - **App Description**: Optional
4. Click "Create"
5. Note the **Client ID** from the created app

## Using Custom Client IDs

To use your own client IDs during development, modify the constants in the backend:

### For AniList

Edit `backend/server/src/track/auth/anilist.rs`:

```rust
const ANILIST_CLIENT_ID: &str = "YOUR_CLIENT_ID_HERE";
```

### For MyAnimeList

Edit `backend/server/src/track/auth/mal.rs`:

```rust
const MAL_CLIENT_ID: &str = "YOUR_CLIENT_ID_HERE";
```

After changing the client IDs, rebuild the backend:

```bash
cd backend
cargo build --release
```

## Testing the OAuth Flow

### On Linux (Bridge Mode)

1. Start the backend server:
   ```bash
   nix run .#start
   ```

2. Open KOReader with the plugin:
   ```bash
   nix run .#dev
   ```

3. In KOReader, go to the RakuYomi plugin → Settings → Tracking
4. Click "Log in" for AniList or MyAnimeList
5. A QR code will appear on screen
6. Scan the QR code with your phone
7. Authorize the app on your phone
8. Copy the token/code from the redirect URL and paste it back in KOReader

### Verifying Tokens

After successful login, you can verify the token was stored:

```bash
# Query the SQLite database
sqlite3 ~/.local/share/koreader/rakuyomi/database.db \
  "SELECT tracker_id FROM tracker_auth"
```

## Troubleshooting

### "Invalid client_id" Error

- Verify the client ID is correct in the source code
- Ensure you rebuilt the backend after changing the client ID
- Check that the OAuth app is still active on the tracker's developer portal

### QR Code Not Scannable

- The QR code is generated as a 300×300 PNG
- Ensure your e-ink device's screen is clean and well-lit
- Try adjusting the screen contrast/brightness
- The QR code encodes the full OAuth URL, which can be long

### Token Exchange Fails (MAL)

MAL uses PKCE, which requires the `code_verifier` to match the `code_challenge` sent during auth URL generation. If you see "token exchange failed":

- Ensure you're using the same session (don't restart the backend between QR scan and token submission)
- The PKCE session expires after 15 minutes
- Check the backend logs with `RUST_LOG=debug` for detailed error messages

### Rate Limiting

Both trackers enforce rate limits:
- **AniList**: 85 requests per minute
- **MAL**: Undocumented, but be respectful

If you're hitting rate limits during development:
- Register your own OAuth app (separate rate limit bucket)
- Add delays between test requests
- Use the production client IDs sparingly

## Production Considerations

The hardcoded client IDs in the repository are the **production** credentials. When deploying RakuYomi to end users:

- Do NOT change the client IDs unless you've registered your own apps
- The production client IDs are already approved by AniList and MAL
- Users should see "RakuYomi" as the app name during OAuth (not your dev app name)

If you fork RakuYomi and distribute it publicly, you **must** register your own OAuth apps to avoid conflicts with the upstream project.

## See Also

- [TRACKING-SYNC-PLAN.md](../../.planning/TRACKING-SYNC-PLAN.md) — Technical design for tracker integration
- [Setting up the Environment](./setting-up-the-environment.md) — General dev environment setup
