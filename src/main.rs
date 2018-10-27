extern crate ww_subscription;
extern crate diesel;
extern crate rss;
extern crate telebot;
extern crate tokio;
extern crate tokio_core;
extern crate futures;

use ww_subscription::*;
use ww_subscription::models::*;

use std::thread;
use std::time::Duration;
use std::env;

use telebot::bot;
use tokio_core::reactor::Core;
use futures::stream::Stream;
use futures::sync::mpsc;

use tokio::prelude::*;
use diesel::prelude::*;
use telebot::functions::*;

fn telegram_bot(receiver: mpsc::UnboundedReceiver<(i32, String)>) {
    thread::spawn(move || {
        let mut lp = Core::new().unwrap();
        let bot = bot::RcBot::new(lp.handle(), &env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN env var not set"))
            .update_interval(200);

        let conn = establish_connection();

        let handle = bot.new_cmd("/subscribe")
            .and_then(move |(bot, msg)| {
                use ww_subscription::schema::subscriptions::dsl::*;

                if msg.text.is_none() {
                    return bot.message(msg.chat.id, "Expected text, found none".into()).send();
                }

                let text = msg.text.unwrap();

                let result = diesel::insert_into(subscriptions)
                .values(&NewSubscription {
                    chat_id: msg.chat.id as i32,
                    novel: &text
                })
                .execute(&conn);

                match result {
                    Ok(_) => {
                        println!("Successfully inserted subscription from chat_id={} to novel={}", msg.chat.id, text);
                        bot.message(msg.chat.id, "Success".into()).send()
                    },
                    Err(e) => {
                        println!("Failed to insert subscription. err={:?}", e);
                        bot.message(msg.chat.id, "Fail".into()).send()
                    }
                }
            });

        bot.register(handle);

        let bot_clone = bot.clone();
        lp.handle().spawn(receiver
            .and_then(move |(chat, novel): (i32, String)| {
                bot_clone.message(chat as i64, novel).send()
                    .map(|_| ())   
                    .map_err(|_| ())
                    .into_future()
            })
            .map_err(|e| panic!("error={:?}", e))
            .for_each(|_| Ok(())));

        bot.run(&mut lp).unwrap();
    });
}

fn main() {
    use ww_subscription::schema::novels::dsl::*;

    let connection = establish_connection();
    let (sender, receiver) = mpsc::unbounded();

    telegram_bot(receiver);

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

            {
                use ww_subscription::schema::subscriptions;
                let subs = subscriptions::table.filter(subscriptions::novel.eq(&new_novel.name)).load::<Subscription>(&connection).expect("Failed to select subscriptions");

                for sub in subs {
                    sender
                        .unbounded_send((sub.chat_id, format!("New chapter released for novel {}.\nLink: {}", new_novel.name, new_novel.last_link)))
                        .expect(&format!("Failed to send notification to user={} for novel={}", sub.chat_id, sub.novel));
                }
            }
        }

        thread::sleep(Duration::from_secs(10));
    }
}
