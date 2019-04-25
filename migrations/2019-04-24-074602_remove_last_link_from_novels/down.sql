-- This file should undo anything in `up.sql`
ALTER TABLE novels
ADD COLUMN IF NOT EXISTS last_link TEXT NOT NULL
