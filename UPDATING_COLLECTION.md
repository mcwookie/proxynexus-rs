# Updating a game collection (Marvel Champions / Arkham Horror LCG)

Reference for two recurring tasks: adding a new expansion when it's
released, and fixing naming mistakes or missing cards you find later.
Applies to both the Marvel Champions (`marvel_champions`) and Arkham
Horror LCG (`ahlcg`) collections — substitute the game id and `.pnx`
filename for whichever you're updating.

Both funnel into the same underlying cycle, because `collection add`
**refuses to run if a collection with that name already exists** (it
errors with "Collection 'x' has already been added" rather than updating
in place):

```
rebuild the .pnx  ->  remove the old collection  ->  add the new one
```

## Adding a new expansion

1. **Refresh the catalog:**
   ```bash
   ./target/release/proxynexus-cli catalog update
   ```
   This re-syncs **all** registered games in one pass (there's no
   `--game` filter) — both adapters fetch live from their API rather than
   hardcoding pack/card data, so new expansions show up here
   automatically as soon as MarvelCDB/ArkhamDB add them, no code changes
   needed. Expect this to take a few minutes: both the Marvel Champions
   and Arkham Horror LCG adapters fetch pack-by-pack rather than in bulk
   (see the "Known pitfalls" note on the bulk endpoints below), so it's
   ~60-115 sequential requests per game rather than one.

2. **Scan/organize the new expansion's physical cards** into a source
   image folder, named `{card_id}@{pack_id}[~back].jpg`/`.png` per the
   naming convention documented in the CLI's README (Image File Naming
   Convention section) —
   `card_id` and `pack_id` must exactly match the official API's codes
   (MarvelCDB or ArkhamDB, whichever game this is).

   - **Marvel Champions**: a companion Python script
     (`rename_marvel_champions.py`, kept outside this repo) fuzzy-matches
     scanned card filenames against the catalog:
     ```bash
     python3 rename_marvel_champions.py --source "path/to/new-expansion-folder" --output ./renamed
     ```
     Pointing `--source` at just the new expansion's folder is faster than
     a full re-run, but re-running against your whole collection folder is
     also completely safe -- already-matched files just get
     re-converted/overwritten harmlessly, just slower. Review
     `match_log.csv` and use `--review` for anything unmatched:
     ```bash
     python3 rename_marvel_champions.py --source "path/to/new-expansion-folder" --output ./renamed --review match_log.csv
     ```
   - **Arkham Horror LCG**: no equivalent matching script exists yet.
     Images need to already be named per the convention above using
     ArkhamDB's card codes (visible in each card's URL, e.g.
     `arkhamdb.com/card/01001`) before `collection build`.

3. **Rebuild the `.pnx`:**
   ```bash
   ./target/release/proxynexus-cli collection build --game <game_id> --images ./renamed --output <game_id>.pnx
   ```
   e.g. `--game marvel_champions --output marvel_champions.pnx` or
   `--game ahlcg --output ahlcg.pnx`. If your image folder is cumulative
   (old + new images all sit together), this produces one complete,
   updated bundle -- not just the new cards.

4. **Remove the old collection** (required -- see note above):
   ```bash
   ./target/release/proxynexus-cli collection remove <game_id>
   ```
   Prompts for a `(y/N)` confirmation.

5. **Add the rebuilt collection:**
   ```bash
   ./target/release/proxynexus-cli collection add <game_id>.pnx
   ```
   Verify with:
   ```bash
   ./target/release/proxynexus-cli query --list-sets -g <game_id>
   ```
   The new expansion's set should now show real printing counts instead
   of "no printings available".

6. **If you're using the self-hosted Docker web app too**, it needs its
   own refresh (see `SETUP.md` in the Docker package for the full
   troubleshooting context on any of this):
   ```bash
   ./target/release/proxynexus-cli export --output data/init.sql
   cp -r ~/.proxynexus/collections/. data/collections/
   docker compose build web --no-cache
   docker compose up -d
   ```
   If Docker runs on a different machine than the one with the images,
   see the "Multi-machine setups" note below before assuming step 6
   alone is enough.

## Correcting mistakes / adding missing cards

Simpler -- you're not touching the catalog at all, just the image files.

1. **Wrong match**: rename the file directly in `./renamed`, e.g.:
   ```bash
   mv 01138@core.jpg 01094@core.jpg
   ```
   Note: `--review` (Marvel Champions script only) only surfaces things it
   *failed* to match, not ones it *confidently matched wrong* -- a
   genuine mismatch needs a manual fix, not another review pass.

2. **Missing card**: add a correctly-named `{card_id}@{pack_id}.jpg` file
   directly into `./renamed`. To find the right `card_id`/`pack_id`,
   search the card on the game's official database — its code appears in
   the card's URL (e.g. `marvelcdb.com/card/21138a` or
   `arkhamdb.com/card/01001`).

3. **Rebuild and swap in**, same three commands as the expansion workflow:
   ```bash
   ./target/release/proxynexus-cli collection build --game <game_id> --images ./renamed --output <game_id>.pnx
   ./target/release/proxynexus-cli collection remove <game_id>
   ./target/release/proxynexus-cli collection add <game_id>.pnx
   ```

4. Update the Docker web app too if you're using it (same step 6 as above).

## Known pitfalls (hit for real while adding Arkham Horror LCG)

- **The bulk `/api/public/cards/` endpoint is unreliable for both games**
  -- confirmed for MarvelCDB (drops some encounter cards) and, worse, for
  ArkhamDB (returned 1,983 cards total against packs.json's summed
  `total` of 8,422). Both adapters fetch pack-by-pack instead, which is
  why `catalog update` takes a few minutes rather than seconds.

- **Non-`.jpg`/`.jpeg`/`.png` image files are silently dropped by
  `collection build`** -- `.webp` in particular, if your source images
  come from a scraper or mod dump. If a card's front gets dropped this
  way but its `~back` sibling survives (or vice versa), `collection add`
  will fail per-card with `"Validation error: Card 'X' (Y) has auxiliary
  parts but no 'front' image."` -- and it'll do this one card at a time
  as you fix each one, not all at once. Before building, check for
  stragglers:
  ```bash
  find ./renamed -type f ! -iname "*.jpg" ! -iname "*.jpeg" ! -iname "*.png"
  ```
  and convert them (e.g. `convert card.webp card.jpg`) before running
  `collection build`.

- **A failed `collection add` can still leave a "ghost" collection
  behind.** The `INSERT INTO collections` row write happens *before* the
  front/back validation check, and isn't wrapped in the same transaction
  -- so a validation failure (like the one above) still marks the
  collection as "added" in the local database, with zero printings
  actually inserted. If `collection add` fails partway through, don't
  just fix the images and retry -- you'll get `"Collection 'x' has
  already been added"` even though `query --list-sets` shows no
  printings. Run `collection remove <game_id>` first, then retry.

## Multi-machine setups (e.g. images on one PC, Docker on another host)

**`collection build`, `collection add`, and `export` must all run on the
same machine** -- specifically, whichever one has the card images and
ran `collection add`. `export` just dumps whatever local `~/.proxynexus`
database exists on the machine you run it on; the catalog re-populates
itself from the network anywhere, but **collections do not** -- they only
exist on the machine where `collection add` actually ran. Running
`export` on a different machine (e.g. directly on the Docker host)
produces a valid-looking `init.sql` with zero collections in it, and the
web app will show sets but every card will be a gray box.

Only `docker compose build`/`up` need to run on the actual Docker host --
transfer `data/init.sql`, `data/collections/`, and (if the game/adapter
code itself changed) the updated `proxynexus-rs/` source tree to the
host, then run the Docker commands there. See `SETUP.md`'s "Important:
export must run on the machine that has the collection" section for the
full detail.

## Why the filename matters

`collection_name` comes from the `.pnx` file's **filename**, not
anything inside it. As long as you keep calling it `<game_id>.pnx` every
time (e.g. always `marvel_champions.pnx`, always `ahlcg.pnx`), the
remove-then-add cycle stays predictable -- `collection remove <game_id>`
will always target the right one.
