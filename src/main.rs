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

// import all available functions
use telebot::functions::*;

fn telegram_bot(receiver: mpsc::UnboundedReceiver<(i64, Novel)>) {
    thread::spawn(move || {
        let mut lp = Core::new().unwrap();
        let bot = bot::RcBot::new(lp.handle(), &env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN env var not set"))
            .update_interval(200);

        let handle = bot.new_cmd("/subscribe")
            .and_then(move |(bot, msg)| {
                println!("{:?}", msg);
                Ok(())
            });

        bot.register(handle);

        let bot_clone = bot.clone();
        lp.handle().spawn(receiver
            .and_then(move |(chat, novel): (i64, Novel)| {
                bot_clone.message(chat, novel.name).send()
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
        }

        thread::sleep(Duration::from_secs(10));
    }
}
