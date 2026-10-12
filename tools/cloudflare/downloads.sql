-- How often each installer and install script was asked for on agentmoshpit.com.
-- One row for each day, file, way it was asked for and country: a number, and nothing
-- about who asked. Written by tools/cloudflare/site.js; read by tools/download-counts.mjs.
--
--   npx wrangler d1 execute agentmoshpit-downloads --remote --file tools/cloudflare/downloads.sql
CREATE TABLE IF NOT EXISTS downloads (
  day TEXT NOT NULL,      -- 2026-10-12, by the clock in UTC
  file TEXT NOT NULL,     -- agent-moshpit_windows_x64-setup.exe, install.sh
  via TEXT NOT NULL,      -- page, direct, script, read or bot (see `via` in site.js)
  country TEXT NOT NULL,  -- two letters, or empty when Cloudflare does not say
  count INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (day, file, via, country)
);
