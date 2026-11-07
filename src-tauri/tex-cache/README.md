# Bundled TeX packages

The installers ship a Tectonic package cache here, so the built-in templates compile
without internet. It's generated, not kept in git:

```bash
scripts/fetch-tectonic.sh
node scripts/build-tex-cache.mjs
```

Without it (a plain development checkout) the app simply uses Tectonic's own
per-user cache and downloads packages on first use.
