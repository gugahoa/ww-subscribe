use super::schema::*;

#[derive(Queryable)]
pub struct Novel {
    pub id: i32,
    pub name: String,
}

#[derive(Insertable)]
#[table_name = "novels"]
pub struct NewNovel<'a> {
    pub name: &'a str,
}

#[derive(Insertable)]
#[table_name = "subscriptions"]
pub struct NewSubscription<'a> {
    pub novel: &'a str,
    pub chat_id: i32,
}

#[derive(Queryable)]
pub struct Subscription {
    pub id: i32,
    pub chat_id: i32,
    pub novel: String,
}

#[derive(Queryable)]
pub struct NovelHistory {
    pub id: i32,
    pub novel_id: i32,
    pub link: String,
}

#[derive(Insertable)]
#[table_name = "novel_history"]
pub struct NewNovelHistory {
    pub link: String,
    pub novel_id: i32,
}
