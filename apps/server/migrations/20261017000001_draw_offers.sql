-- Draw offers in games against other players (live and turn by turn). The latest offer is
-- kept (who, at which ply, whether it's still open) so a player can't repeat an offer every
-- move: the same player may offer again only `DRAW_OFFER_GAP_PLIES` later. Any move closes an
-- open offer.
ALTER TABLE online_games
    ADD COLUMN draw_offer_by   text,
    ADD COLUMN draw_offer_ply  integer,
    ADD COLUMN draw_offer_open boolean NOT NULL DEFAULT false,
    ADD CONSTRAINT online_games_draw_offer_by CHECK (draw_offer_by IN ('south', 'north')),
    ADD CONSTRAINT online_games_draw_offer_open CHECK (NOT draw_offer_open OR draw_offer_by IS NOT NULL);
