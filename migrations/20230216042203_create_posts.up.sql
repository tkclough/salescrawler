-- Add up migration script here
CREATE TABLE IF NOT EXISTS posts 
(
    id TEXT PRIMARY KEY NOT NULL,
    created_utc TEXT NOT NULL,
    downs INTEGER,
    link_flair_text TEXT,
    title TEXT NOT NULL,
    ups INTEGER,
    url TEXT
);

CREATE TABLE IF NOT EXISTS parsed_titles (
    post_id TEXT REFERENCES posts (id),
    product_type TEXT NOT NULL,
    description TEXT NOT NULL,
    price_dollars INTEGER NOT NULL,
    price_cents INTEGER NOT NULL,
    extra_details TEXT
);

CREATE TABLE IF NOT EXISTS rules (
    id TEXT PRIMARY KEY,
    name TEXT,
    link_flair_pattern TEXT,
    product_type_pattern TEXT,
    description_pattern TEXT,
    price_min REAL,
    price_max REAL
);

CREATE TABLE IF NOT EXISTS rule_matches (
    rule_id TEXT NOT NULL REFERENCES rules (id),
    post_id TEXT NOT NULL REFERENCES posts (id),
    created_utc TEXT NOT NULL
);