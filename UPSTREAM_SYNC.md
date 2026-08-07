# Syncing upstream changes

Reference for pulling in updates from the original repo
(`axmccx/proxynexus-rs`) into this fork (`mcwookie/proxynexus-rs`),
without having to re-derive the workflow each time.

## One-time setup (already done, kept here for reference)

```bash
git remote add upstream https://github.com/axmccx/proxynexus-rs.git
git remote -v   # should show both origin (your fork) and upstream
```

## Each time you want to sync

1. **Fetch upstream's latest commits** (doesn't touch your working tree yet):
   ```bash
   git fetch upstream
   ```

2. **See what actually changed before merging anything in:**
   ```bash
   git log HEAD..upstream/master --oneline
   ```
   Worth a quick skim, especially for anything touching:
   - `proxynexus-core/src/games/mod.rs`
   - `proxynexus-core/src/catalog.rs`
   - `proxynexus-gui/src/components/mod.rs`
   - `proxynexus-core/src/image_provider.rs`

   These four are exactly the files the Marvel Champions and Arkham
   Horror LCG work also touches (game registration lists, and the
   `PROXYNEXUS_COLLECTIONS_URL` patch for self-hosted Docker deployment
   -- which needs the override applied in *both* `components/mod.rs`
   and `image_provider.rs`, see the "If conflicts appear" note below),
   so that's where conflicts are most likely to show up.

3. **Merge:**
   ```bash
   git merge upstream/master
   ```
   No conflicts → git commits the merge automatically, done.

4. **If conflicts appear:**
   - In `games/mod.rs` or `catalog.rs`: almost always just both sides
     adding a new line to the same list (e.g. a new game's `mod`
     declaration or adapter registration). Keep both your
     `marvel_champions`/`ahlcg` lines and whatever upstream added, then
     `git add <file>`.
   - In `proxynexus-gui/src/components/mod.rs` or
     `proxynexus-core/src/image_provider.rs`: means upstream touched
     `build_image_url` or `RemoteImageProvider` respectively. Manually
     re-merge your `PROXYNEXUS_COLLECTIONS_URL` override logic with
     whatever upstream changed — review carefully, and check **both**
     files regardless of which one conflicted: they need the identical
     override pattern, and having it in only one of them silently breaks
     either preview thumbnails or PDF/MPC generation depending on which
     one's missing (confirmed the hard way — see `SETUP.md`'s
     troubleshooting section, "Clicking Generate does nothing").
   - Finish with `git commit` once all conflicts are resolved.

5. **Push the merge back to your fork:**
   ```bash
   git push origin master
   ```

6. **Rebuild everything that depends on the changed code:**
   ```bash
   cargo build -p proxynexus-cli --release
   ```
   And if `proxynexus-gui` changed at all, rebuild the Docker web app too.
   Step 5's push already triggered CI
   (`.github/workflows/docker-build.yml` in this repo, which checks out
   `proxynexus-rs` fresh on every run) to build and push a new image —
   once that run finishes (check the Actions tab), pull it on the Docker
   host:
   ```bash
   docker compose pull web
   docker compose up -d web
   ```
   Or skip waiting on CI and rebuild locally instead (see `../PACKAGING.md`
   / `SETUP.md` in the Docker package for that flow):
   ```bash
   docker compose build web --no-cache
   docker compose up -d web
   ```

## Housekeeping

`proxynexus-gui/src/components/mod-orig.rs` (a backup made before
patching `build_image_url`) is untracked — either delete it or add it to
`.gitignore` if you want to keep it around locally, so it stops showing
up in `git status`.
