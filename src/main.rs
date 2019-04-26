use ww_subscription::models::*;
use ww_subscription::*;

use std::collections::HashMap;
use std::env;
use std::thread;
use std::time::Duration;

use futures::sync::mpsc;

use diesel::prelude::*;
use rayon::prelude::*;
use telebot::functions::*;
use tokio::prelude::*;

fn main() {
    let connection = establish_connection();
    let (sender, receiver) = mpsc::unbounded();

    telegram_bot(receiver);

    loop {
        let channel = rss::Channel::from_url("https://www.wuxiaworld.com/feed/chapters")
            .expect("Could not fetch RSS feed");

        let novels_map: HashMap<&str, &str> = channel
            .items()
            .into_iter()
            .map(|novel| (novel.categories()[0].name(), novel.link()))
            .filter(|(_name, link)| link.is_some())
            .map(|(name, link)| (name, link.unwrap()))
            .collect();

        let mut existing_novels: HashMap<String, i32> = {
            use ww_subscription::schema::novels::dsl::*;
            novels
                .filter(name.eq_any(novels_map.keys()))
                .load::<Novel>(&connection)
                .expect("Failed to query novels")
                .into_iter()
                .map(|novel| (novel.name, novel.id))
                .collect()
        };

        let sent_novels: Vec<String> = {
            use ww_subscription::schema::novel_history::dsl::*;
            novel_history
                .filter(link.eq_any(novels_map.values()))
                .load::<NovelHistory>(&connection)
                .expect("Failed to query novel history")
                .into_iter()
                .map(|novel| novel.link)
                .collect()
        };

        let inserted_novels: HashMap<String, i32> =
            novels_map
                .keys()
                .into_iter()
                .map(|novel| String::from(*novel))
                .filter(|novel| !existing_novels.contains_key(novel))
                .map(|novel| {
                    let new_novel = NewNovel { name: &novel };

                    use ww_subscription::schema::novels::dsl::*;
                    println!("Inserting new novel: {}", novel);
                    diesel::insert_into(novels)
                        .values(&new_novel)
                        .returning(id)
                        .get_results(&connection)
                        .ok()
                        .and_then(|result| result.into_iter().next())
                        .map(|novel_id| (novel, novel_id))
                })
                .filter(Option::is_some)
                .map(Option::unwrap)
                .collect();

        existing_novels.extend(inserted_novels);
        novels_map.par_iter().for_each(|(novel, link)| {
            if sent_novels.contains(&(*link).to_owned()) {
                return;
            }

            let connection = establish_connection();
            use ww_subscription::schema::novel_history;
            let result = diesel::insert_into(novel_history::table)
                .values(&NewNovelHistory {
                    novel_id: existing_novels.get(*novel).unwrap().clone(),
                    link: (*link).to_owned(),
                })
                .execute(&connection)
                .map(|_| {
                    use ww_subscription::schema::subscriptions;
                    let subs = subscriptions::table
                        .filter(subscriptions::novel.eq(novel))
                        .load::<Subscription>(&connection)
                        .expect("Failed to select subscriptions");

                    subs.par_iter().for_each(|sub| {
                        sender
                            .unbounded_send((
                                sub.chat_id,
                                format!("New chapter released for novel {}\nLink: {}", novel, link),
                            ))
                            .expect(&format!(
                                "Failed to send notification to user={} for novel={}",
                                sub.chat_id, sub.novel
                            ));
                    });
                });

            match result {
                Ok(_) => println!(
                    "Created new novel history for novel {} link {}",
                    novel, link
                ),
                Err(err) => eprintln!(
                    "Failed to create new novel history for novel {}, err = {}",
                    novel, err
                ),
            }
        });

        thread::sleep(Duration::from_secs(10));
    }
}
