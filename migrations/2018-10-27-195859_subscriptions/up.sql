-- Your SQL goes here
CREATE TABLE subscriptions (
    id SERIAL PRIMARY KEY,
    chat_id INTEGER NOT NULL,
    novel VARCHAR(64) NOT NULL,

    CONSTRAINT novels_subscriptions FOREIGN KEY (novel) REFERENCES novels (name),
    UNIQUE (chat_id, novel)
)