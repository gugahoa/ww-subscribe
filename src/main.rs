extern crate ww_subscription;
extern crate diesel;
extern crate rss;

use ww_subscription::*;
use ww_subscription::models::*;
use diesel::prelude::*;
use std::thread;
use std::time::Duration;

fn main() {
    use ww_subscription::schema::novels::dsl::*;

    let connection = establish_connection();

    loop {
        let channel = rss::Channel::from_url("https://www.wuxiaworld.com/feed/chapters").expect("Could not fetch RSS feed");

        for novel in channel.into_items() {
            let new_novel = NewNovel {
                name: novel.categories()[0].name(),
                last_link: novel.link().expect(&format!("Could not get item link. {:?}", &novel))
            };

            let last_novel = novels.filter(last_link.eq(new_novel.last_link)).load::<Novel>(&connection).expect("Failed to query novels");
            // Last link has already been seen, so we don't have new chapters
            if last_novel.len() > 0 {
                println!("Already fetched all updates");
                break;
            }

            println!("Inserting novel: {}, last_link: {}", new_novel.name, new_novel.last_link);

            // upsert new chapters, to keep track of what we have already notified users
            diesel::insert_into(novels)
            .values(&new_novel)
            .on_conflict(name)
            .do_update()
            .set(last_link.eq(novel.link().expect(&format!("Could not get item link. {:?}", &novel))))
            .execute(&connection)
            .expect("Failed to insert or update item");
        }

        thread::sleep(Duration::from_secs(10));
    }
}
