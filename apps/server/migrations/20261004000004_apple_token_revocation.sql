-- Sign in with Apple token revocation on account deletion (ADR 0009). At sign-in the server
-- exchanges Apple's authorization code for Apple's refresh token and keeps it here, with the
-- client ID it was issued to (Services ID for the web, bundle ID for iOS), only so it can be
-- revoked when the account is deleted. Expand-only: both columns are nullable.
ALTER TABLE auth_identities
    ADD COLUMN provider_refresh_token text,
    ADD COLUMN provider_client_id     text;
