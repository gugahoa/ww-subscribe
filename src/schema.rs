table! {
    novel_history (id) {
        id -> Int4,
        novel_id -> Int4,
        link -> Text,
    }
}

table! {
    novels (id) {
        id -> Int4,
        name -> Varchar,
    }
}

table! {
    subscriptions (id) {
        id -> Int4,
        chat_id -> Int4,
        novel -> Varchar,
    }
}

joinable!(novel_history -> novels (novel_id));

allow_tables_to_appear_in_same_query!(novel_history, novels, subscriptions,);
