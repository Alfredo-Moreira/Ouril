-- Field-level last-writer-wins for settings by device time (data-and-sync.md).
-- `field_times` maps each settings field to the (clamped) client_time of the change that set
-- it, e.g. {"sound": "2026-10-04T09:00:00Z"}. A pushed change only overwrites a field if it is
-- at least as recent, so a mutation that was deferred and applied later can't undo a newer one.
-- Expand-only: existing rows start with no times, so their next change of each field applies.
ALTER TABLE user_settings
    ADD COLUMN field_times jsonb NOT NULL DEFAULT '{}'::jsonb;
