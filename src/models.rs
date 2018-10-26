use super::schema::novels;

#[derive(Queryable)]
pub struct Novel {
    pub id: i32,
    pub name: String,
    pub last_link: String
}

#[derive(Insertable)]
#[table_name="novels"]
pub struct NewNovel<'a> {
    pub name: &'a str,
    pub last_link: &'a str
}