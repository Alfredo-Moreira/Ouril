-- Field-level last-writer-wins for profile fields by device time (data-and-sync.md), like
-- user_settings.field_times. Maps each field (display_name, handle, avatar_url, locale,
-- country) to the time of the change that set it: the clamped client_time for synced changes,
-- server time for direct PATCH /v1/me edits. An older change never overwrites a newer field.
-- Expand-only: existing rows start with no times, so their next change of each field applies.
ALTER TABLE users
    ADD COLUMN profile_field_times jsonb NOT NULL DEFAULT '{}'::jsonb;
