-- Your SQL goes here
CREATE TABLE novel_history (
  id SERIAL PRIMARY KEY,
  novel_id SERIAL REFERENCES novels(id) NOT NULL,
  link TEXT NOT NULL
);
