-- Your SQL goes here
CREATE TABLE novels (
    id SERIAL PRIMARY KEY,
    name VARCHAR(32) UNIQUE NOT NULL,
    last_link TEXT NOT NULL
)