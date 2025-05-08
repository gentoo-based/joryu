-- Add migration script here-- migrations/YYYYMMDDHHMMSS_create_guild_prefixes_table.sql

-- Create the table to store guild prefixes
CREATE TABLE guild_prefixes (
    guild_id INTEGER PRIMARY KEY, -- SQLite uses INTEGER PRIMARY KEY
    prefix VARCHAR(10) NOT NULL DEFAULT '!'
);

-- You can also add a "down" migration here for rolling back changes
-- DROP TABLE guild_prefixes;
