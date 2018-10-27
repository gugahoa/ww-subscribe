table! {
    novels (id) {
        id -> Int4,
        name -> Varchar,
        last_link -> Text,
    }
}

table! {
    subscriptions (id) {
        id -> Int4,
        chat_id -> Int4,
        novel -> Varchar,
    }
}

allow_tables_to_appear_in_same_query!(
    novels,
    subscriptions,
);
