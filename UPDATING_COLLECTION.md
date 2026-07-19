# Updating the Marvel Champions collection

Reference for two recurring tasks: adding a new expansion when it's
released, and fixing naming mistakes or missing cards you find later.

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
   The adapter fetches live from MarvelCDB's API rather than hardcoding
   pack/card data, so new expansions show up here automatically as soon
   as MarvelCDB adds them -- no code changes needed for this step.

2. **Scan/organize the new expansion's physical cards** into a source
   image folder, same structure as before (folder name roughly matching
   the pack, card titles embedded in filenames).

3. **Run the rename script against the new cards:**
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

4. **Rebuild the `.pnx`:**
   ```bash
   ./target/release/proxynexus-cli collection build --game marvel_champions --images ./renamed --output marvel_champions.pnx
   ```
   Since `./renamed` is cumulative (old + new images all sit there
   together), this produces one complete, updated bundle -- not just the
   new cards.

5. **Remove the old collection** (required -- see note above):
   ```bash
   ./target/release/proxynexus-cli collection remove marvel_champions
   ```
   Prompts for a `(y/N)` confirmation.

6. **Add the rebuilt collection:**
   ```bash
   ./target/release/proxynexus-cli collection add marvel_champions.pnx
   ```
   Verify with:
   ```bash
   ./target/release/proxynexus-cli query --list-sets -g marvel_champions
   ```
   The new expansion's set should now show real printing counts instead
   of "no printings available".

7. **If you're using the self-hosted Docker web app too**, it needs its
   own refresh (see `SETUP.md` in the Docker package for the full
   troubleshooting context on any of this):
   ```bash
   ./target/release/proxynexus-cli export --output data/init.sql
   cp -r ~/.proxynexus/collections/. data/collections/
   docker compose build web --no-cache
   docker compose up -d
   ```

## Correcting mistakes / adding missing cards

Simpler -- you're not touching the catalog at all, just the image files.

1. **Wrong match**: rename the file directly in `./renamed`, e.g.:
   ```bash
   mv 01138@core.jpg 01094@core.jpg
   ```
   Note: `--review` only surfaces things the script *failed* to match,
   not ones it *confidently matched wrong* -- a genuine mismatch needs a
   manual fix, not another review pass.

2. **Missing card**: add a correctly-named `{card_id}@{pack_id}.jpg` file
   directly into `./renamed`. To find the right `card_id`/`pack_id`,
   search the card on marvelcdb.com -- the code appears in the card's
   URL (e.g. `marvelcdb.com/card/21138a`).

3. **Rebuild and swap in**, same three commands as the expansion workflow:
   ```bash
   ./target/release/proxynexus-cli collection build --game marvel_champions --images ./renamed --output marvel_champions.pnx
   ./target/release/proxynexus-cli collection remove marvel_champions
   ./target/release/proxynexus-cli collection add marvel_champions.pnx
   ```

4. Update the Docker web app too if you're using it (same step 7 as above).

## Why the filename matters

`collection_name` comes from the `.pnx` file's **filename**, not
anything inside it. As long as you keep calling it
`marvel_champions.pnx` every time, the remove-then-add cycle stays
predictable -- `collection remove marvel_champions` will always target
the right one.
