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
   (MarvelCDB or ArkhamDB, whichever game this is). **The `~back` part
   applies to both games** — see "Double-sided cards: two different
   APIs, one convention" below before naming any double-sided card's
   files (Marvel Champions' hero/alter-ego pairing needs it too, not
   just Arkham Horror LCG's investigators).

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
   - **Arkham Horror LCG**: not a fuzzy-matcher for scans -- instead,
     `lcg_tts_processor.py` (kept outside this repo, in
     `lcg-utils/lcg-tts-processor/`) extracts card images directly from
     Tabletop Simulator save data (the
     [SCED](https://github.com/Chr1Z93/SCED) mod for player cards,
     [SCED-downloads](https://github.com/Chr1Z93/SCED-downloads) for
     encounter cards) and identifies each card against ArkhamDB itself,
     writing output already in the naming convention above via its
     `--naming dbid` flag:
     ```bash
     python3 lcg_tts_processor.py SAVEFILE.json -o ./renamed --game arkham --naming dbid
     ```
     Test with `--limit 20` before committing to a full run. See that
     script's own `README.md`/`PROJECT_CONTEXT.md` for the input file
     details (which save to point it at, how to compose one from
     SCED-downloads' decomposed format) and a running log of real bugs
     hit and fixed -- several of which produced exactly the kind of
     silent, hard-to-spot-in-the-output problems this "Known pitfalls"
     section is about (a bulk-vs-per-pack API completeness question, and
     minicard variants silently winning over the real card).

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
   ```
   `data/collections` is never tracked in git (see `.gitignore` -- it's
   pure image data, GBs in size) and never goes near CI either way; that
   copy always has to happen locally like this. `data/init.sql` **is**
   tracked, though, which changes how the last step works:

   - **Commit and push `data/init.sql`** (recommended) -- CI
     (`.github/workflows/docker-build.yml`) builds a fresh image with
     the new catalog automatically. Once that run finishes (check the
     Actions tab), on the Docker host: `docker compose pull web &&
     docker compose up -d`.
   - **Or skip CI and rebuild locally** if you need it live immediately
     and don't want to wait on a CI run: `docker compose build web
     --no-cache && docker compose up -d`.

   Either way, if any card images changed (not just the catalog), also
   re-mirror them into MinIO -- neither path above touches that:
   `docker compose up -d --force-recreate minio-init`.

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

## Double-sided cards: two different APIs, one convention

Marvel Champions and Arkham Horror LCG represent a double-sided card
differently at the source API, but **both need the same `~back`
convention** -- an earlier version of this doc got Marvel Champions
wrong here (said "never use `~back`"), which was backwards. Corrected
after user pushback plus confirming the actual rules: a Marvel
Champions hero's Hero/Alter-Ego forms are one physical card players
flip during play, exactly like an ArkhamDB investigator -- not two
separate cards, despite MarvelCDB's API making it look that way. See
the main `README.md`'s "Marvel Champions hero/alter-ego: one physical
card, not two" section for the full writeup.

- **Arkham Horror LCG (ArkhamDB)**: a double-sided card (most
  investigators, acts, agendas) has **one** card code with separate
  `imagesrc`/`backimagesrc` fields. Name the files
  `{card_id}@{pack_id}.jpg` (front) and `{card_id}@{pack_id}~back.jpg`
  (back).

- **Marvel Champions (MarvelCDB)**: a double-sided card (hero/alter-ego
  pairs, and some Main Scheme A/B sides) gets **two separate card
  codes** from MarvelCDB's raw API, e.g. `01001a` (Spider-Man) and
  `01001b` (Peter Parker) -- but only `01001a` is a real catalog entry
  (`01001b` is MarvelCDB's `hidden: true` side and gets folded into
  `01001a`'s `linked_card_*` metadata instead, same as `01001` never
  being its own entry). Name the files `01001a@core.jpg` (front, Hero
  art) and `01001a@core~back.jpg` (back, Alter-Ego art) -- **never**
  `01001b@core.jpg`; there's no catalog entry for the raw `b` code, so a
  file named that way silently fails to match any official printing.
  `rename_marvel_champions.py` (the companion scanning tool) handles
  this pairing automatically -- see its own doc for how.

## The same card showing up more than once within one pack/set

Not a bug -- confirmed by tracing the actual query (`SetName`'s
`get_card_requests_from_set_name` in `card_store.rs`) and the raw
catalog data behind it, after "why does Tommy Muldoon's set show M1911
and Police Dog twice?" turned out to have a real, verifiable answer
rather than being pipeline damage. That query joins
`cards`/`card_versions`/`packs` strictly on `pack.name == <requested
set>` and repeats each resulting row by its `quantity` field -- it does
**no** deduplication by title. Two different, unrelated reasons a card
can end up appearing more than once for the same reason a human would
expect only one:

- **Ordinary quantity > 1** -- one `card_id`, one `card_versions` row for
  that pack, `quantity` field is 2 (or more). This is the common case
  (e.g. 2x Iron Sights, 4x Physical Fitness) -- no different from any
  deck legitimately containing multiple copies of the same card.
- **Two separate official card codes under the same pack** -- rarer, but
  real: e.g. Tommy Muldoon's own starter-deck pack (`tom`) has *two*
  distinct ArkhamDB codes both named "M1911" (`60155` and `60171`, each
  with its own `quantity`), and likewise for "Police Dog". ArkhamDB
  apparently assigns sequential codes per physical card slot for these
  small investigator-starter products rather than one code with a higher
  quantity. Confirmed via the exported catalog directly:
  ```bash
  python3 -c "
  import gzip, re
  content = gzip.open('data/init.sql', 'rt').read()
  ids = re.findall(r\"\('(ahlcg_[^']*)', '[^']*', 'ahlcg', 'M1911',\", content)
  print(ids)"
  ```
  Both mechanisms are correct, official-data behavior, not something to
  "fix" -- generating the set really does produce that many cards, and a
  real physical copy of the product has that many too.

This is a distinct situation from "Double-sided cards" above (front/back
of one physical card) and from a genuinely reprinted investigator
appearing under a *different* pack (Carolyn Fern in both `car` and `tcu`
-- see the query's `# also:` annotation, which only shows up for that
cross-pack case, not the same-pack duplicate-code case above).

## Known pitfalls (hit for real while adding Arkham Horror LCG)

- **A buggy TTS export script can tag every card with a spurious
  `~back` file**, not just genuinely double-sided ones. If your source
  images come from a Tabletop Simulator save exporter, check that it
  only writes a `~back` file when the deck's `UniqueBack` flag is true
  -- if it writes one unconditionally (even a shared, generic
  player/encounter card back for single-sided cards), every single-sided
  card will incorrectly show a card back in the app. Symptom: an
  implausibly high fraction of `~back` files relative to genuinely
  double-sided cards (e.g. ~50% in a collection where only ~25% of cards
  are actually double-sided). To retroactively clean up a collection
  already exported this way, cross-reference each `~back` file's
  `(card_id, pack_id)` against the game API's own `double_sided` field
  (ArkhamDB) rather than relying on image-hash deduplication alone --
  the "generic" back image isn't always byte-identical across packs
  (different scan batches/resolutions), so hash-based dedup under-counts
  the spurious files.

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
  as you fix each one, not all at once. If the front *does* survive
  (just the back is `.webp`) there's no error at all -- the card silently
  builds as single-sided, missing a back it should have had. Confirmed
  recurring: an initial cleanup found and fixed ~150 stray `.webp` files,
  and a later scan of the same (by-then-much-larger) collection turned up
  25 more -- new content merged in later re-introduces the problem, it's
  not a one-time fix. Re-scan the **whole** collection folder before
  every `collection build`, not just newly-added files:
  ```bash
  find /path/to/collection -type f ! -iname "*.jpg" ! -iname "*.jpeg" ! -iname "*.png" ! -iname "manifest.csv"
  ```
  and convert any hits (e.g. `convert card.webp card.jpg`) before
  building.

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

**Check whether "transfer" is even a manual step first.** If
`proxynexus-package/` lives under a network mount (NFS/SMB/etc.) that's
mounted identically on both the machine you build on and the Docker
host, there's nothing to copy -- writes to `data/init.sql`/
`data/collections/` are already visible on the Docker host the moment
they happen. Confirm with `mount | grep <path>` (or compare `df`'s
"Filesystem" column) on both machines -- same server + same export path
means it's one shared filesystem, not two copies. `~/.proxynexus`
(the CLI's own database/collection cache) is a different story: it
lives under the home directory, not typically on a shared mount, so
it's genuinely separate per machine even when `proxynexus-package/`
itself is shared -- which is exactly why `collection add`/`export`
still have to run on whichever machine actually holds the images.

## Why the filename matters

`collection_name` comes from the `.pnx` file's **filename**, not
anything inside it. As long as you keep calling it `<game_id>.pnx` every
time (e.g. always `marvel_champions.pnx`, always `ahlcg.pnx`), the
remove-then-add cycle stays predictable -- `collection remove <game_id>`
will always target the right one.
